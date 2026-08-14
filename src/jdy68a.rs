//! JDY-68A：UART AT + 事件解析 + STAT 引脚监控。
//!
//! 手机以「音频蓝牙」连接时，很多双模模块的 STAT 只表示 BLE，不一定会拉高。
//! 因此连接检测以 UART（+CONNECTING / +CONNECTED / +DISC）和 AT+STAT 轮询为主。

use core::sync::atomic::{AtomicU8, Ordering};

use defmt::warn;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::uarte::{self, Baudrate, Parity, Uarte, UarteRxWithIdle, UarteTx};
use embassy_nrf::{bind_interrupts, Peri};
use embassy_time::{Duration, Timer};

use crate::events::{AudioEvent, AUDIO_EVENTS};
use crate::log_line;
use crate::pins::{
    AUDIO_BT_NAME, BUTTON_DEBOUNCE_MS, JDY_AT_TIMEOUT_MS, JDY_STAT_POLL_MS, STAT_SAMPLE_MS,
    STAT_STABLE_SAMPLES, STAT_STARTUP_DELAY_MS,
};

bind_interrupts!(struct UarteIrqs {
    UARTE0 => uarte::InterruptHandler<embassy_nrf::peripherals::UARTE0>;
});

/// 0=unknown, 1=disconnected, 2=connected — 用于去重连接类事件
static LAST_LINK: AtomicU8 = AtomicU8::new(0);
/// 0=unknown, 1=paused/stopped, 2=playing — 用于去重播放类事件
static LAST_PLAY: AtomicU8 = AtomicU8::new(0);
/// Last +STAT=a,b raw for change-only RX log (packed: high=a, low=b as ascii '0'/'1')
static LAST_STAT_PAIR: AtomicU8 = AtomicU8::new(0xFF);

struct LineBuf {
    data: [u8; 96],
    len: usize,
}

impl LineBuf {
    const fn new() -> Self {
        Self {
            data: [0; 96],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn push(&mut self, byte: u8) -> Option<&str> {
        match byte {
            b'\r' | b'\n' => {
                if self.len == 0 {
                    return None;
                }
                let line = core::str::from_utf8(&self.data[..self.len]).ok()?;
                self.len = 0;
                Some(line)
            }
            _ if self.len < self.data.len() => {
                self.data[self.len] = byte;
                self.len += 1;
                None
            }
            _ => {
                // Overflow: keep last bytes so a long MAC line still ends cleanly next NL
                self.len = 0;
                None
            }
        }
    }
}

fn log_uart_raw(s: &str) {
    let t = s.trim();
    if t.is_empty() {
        return;
    }
    // Link poll replies: only print when +STAT=a,b value changes
    if let Some(rest) = t.strip_prefix("+STAT=") {
        if rest.contains(',') {
            let mut parts = rest.split(',');
            let a = parts.next().and_then(|p| p.trim().chars().next()).unwrap_or('?');
            let b = parts.next().and_then(|p| p.trim().chars().next()).unwrap_or('?');
            if matches!(a, '0' | '1') && matches!(b, '0' | '1') {
                let packed = ((a as u8) << 4) | (b as u8 & 0x0F);
                if LAST_STAT_PAIR.swap(packed, Ordering::AcqRel) == packed {
                    return;
                }
            }
        }
    }
    let mut buf = [0u8; 88];
    let prefix = b"RX> ";
    buf[..prefix.len()].copy_from_slice(prefix);
    let max = buf.len() - prefix.len();
    let n = t.len().min(max);
    buf[prefix.len()..prefix.len() + n].copy_from_slice(&t.as_bytes()[..n]);
    if let Ok(text) = core::str::from_utf8(&buf[..prefix.len() + n]) {
        log_line::line(text);
    }
}

fn parse_jdy_line(line: &str) -> Option<AudioEvent> {
    let line = line.trim();
    if line.starts_with("+CONNECTING")
        || line.starts_with("+CONNECTED")
        || line == "CONNECTED"
    {
        Some(AudioEvent::Connected)
    } else if line.starts_with("+DISC") {
        Some(AudioEvent::Disconnected)
    } else if line.starts_with("+PLAY") {
        // +PLAY=iPhone / +PLAY=OK
        Some(AudioEvent::PlaybackStarted)
    } else if line.starts_with("+PAUSE") {
        Some(AudioEvent::PlaybackPaused)
    } else if let Some(ev) = parse_pstat_query(line) {
        Some(ev)
    } else if let Some(ev) = parse_link_stat_query(line) {
        Some(ev)
    } else {
        None
    }
}

/// AT+PSTAT -> +PSTAT=0|1  or some FW: +STAT=0|1 (single field, no comma)
fn parse_pstat_query(line: &str) -> Option<AudioEvent> {
    let line = line.trim();
    let rest = if let Some(r) = line.strip_prefix("+PSTAT=") {
        r
    } else if let Some(r) = line.strip_prefix("+STAT=") {
        // single-field only; two-field is link status
        if r.contains(',') {
            return None;
        }
        r
    } else {
        return None;
    };
    match rest.trim().chars().next() {
        Some('1') => Some(AudioEvent::PlaybackStarted),
        Some('0') => Some(AudioEvent::PlaybackPaused),
        _ => None,
    }
}

/// AT+STAT -> +STAT=a,b（必须带逗号）。任一字段为 1 = 已连接。
fn parse_link_stat_query(line: &str) -> Option<AudioEvent> {
    let line = line.trim();
    let rest = line.strip_prefix("+STAT=")?;
    if !rest.contains(',') {
        return None;
    }
    let rest = rest.trim().trim_start_matches('<').trim_end_matches('>');
    let mut parts = rest.split(',');
    let a = parts.next()?.trim().chars().next()?;
    let b = parts.next().and_then(|p| p.trim().chars().next()).unwrap_or('0');
    if !matches!(a, '0' | '1') {
        return None;
    }
    if a == '1' || b == '1' {
        Some(AudioEvent::Connected)
    } else {
        Some(AudioEvent::Disconnected)
    }
}

fn parse_nama_value(line: &str) -> Option<&str> {
    let line = line.trim();
    let rest = if let Some(r) = line.strip_prefix("+NAMA=") {
        r
    } else if let Some(r) = line.strip_prefix("+NAMA:") {
        r
    } else {
        return None;
    };
    let name = rest.trim();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn is_ok_line(line: &str) -> bool {
    let t = line.trim();
    t.eq_ignore_ascii_case("OK") || t.eq_ignore_ascii_case("+OK")
}

#[derive(Clone, Copy)]
enum EventSource {
    Stat,
    Uart,
}

fn link_code(event: AudioEvent) -> Option<u8> {
    match event {
        AudioEvent::Disconnected => Some(1),
        AudioEvent::Connected => Some(2),
        _ => None,
    }
}

fn play_code(event: AudioEvent) -> Option<u8> {
    match event {
        AudioEvent::PlaybackPaused => Some(1),
        AudioEvent::PlaybackStarted => Some(2),
        _ => None,
    }
}

async fn publish(event: AudioEvent, source: EventSource) {
    if let Some(code) = link_code(event) {
        if LAST_LINK.swap(code, Ordering::AcqRel) == code {
            return;
        }
    }
    if let Some(code) = play_code(event) {
        if LAST_PLAY.swap(code, Ordering::AcqRel) == code {
            return;
        }
    }

    match (source, event) {
        (EventSource::Stat, AudioEvent::Connected) => log_line::line("STAT: connected"),
        (EventSource::Stat, AudioEvent::Disconnected) => log_line::line("STAT: disconnected"),
        (EventSource::Uart, AudioEvent::Connected) => log_line::line("UART: connected"),
        (EventSource::Uart, AudioEvent::Disconnected) => log_line::line("UART: disconnected"),
        (_, AudioEvent::PlaybackStarted) => log_line::line("UART: play"),
        (_, AudioEvent::PlaybackPaused) => log_line::line("UART: pause"),
    }
    let _ = AUDIO_EVENTS.try_send(event);
}

async fn send_at(tx: &mut UarteTx<'_>, cmd: &str) -> Result<(), uarte::Error> {
    let mut buf = [0u8; 48];
    let cmd_bytes = cmd.as_bytes();
    if cmd_bytes.len() + 2 > buf.len() {
        return Ok(());
    }
    buf[..cmd_bytes.len()].copy_from_slice(cmd_bytes);
    buf[cmd_bytes.len()] = b'\r';
    buf[cmd_bytes.len() + 1] = b'\n';
    let n = cmd_bytes.len() + 2;
    tx.write(&buf[..n]).await
}

async fn flush_rx(rx: &mut UarteRxWithIdle<'_>) {
    let mut chunk = [0u8; 64];
    for _ in 0..4 {
        match select(
            rx.read_until_idle(&mut chunk),
            Timer::after(Duration::from_millis(30)),
        )
        .await
        {
            Either::First(Ok(0)) | Either::Second(()) => break,
            Either::First(Ok(_)) => {}
            Either::First(Err(_)) => break,
        }
    }
}

async fn wait_lines(
    rx: &mut UarteRxWithIdle<'_>,
    line: &mut LineBuf,
    timeout_ms: u64,
    mut on_line: impl FnMut(&str) -> bool,
) -> bool {
    let mut chunk = [0u8; 64];
    let end = embassy_time::Instant::now() + Duration::from_millis(timeout_ms);

    loop {
        let now = embassy_time::Instant::now();
        if now >= end {
            return false;
        }
        let remain = end - now;
        match select(rx.read_until_idle(&mut chunk), Timer::after(remain)).await {
            Either::Second(()) => return false,
            Either::First(Err(_)) => return false,
            Either::First(Ok(0)) => {}
            Either::First(Ok(n)) => {
                for &b in &chunk[..n] {
                    if let Some(s) = line.push(b) {
                        if on_line(s) {
                            return true;
                        }
                    }
                }
            }
        }
    }
}

async fn query_audio_name(
    tx: &mut UarteTx<'_>,
    rx: &mut UarteRxWithIdle<'_>,
    line: &mut LineBuf,
    out: &mut [u8],
) -> Option<usize> {
    line.clear();
    flush_rx(rx).await;
    line.clear();

    if send_at(tx, "AT+NAMA").await.is_err() {
        warn!("AT+NAMA send failed");
        log_line::line("BT name: query send failed");
        return None;
    }

    let mut got_len = None;
    let ok = wait_lines(rx, line, JDY_AT_TIMEOUT_MS, |s| {
        if let Some(name) = parse_nama_value(s) {
            let n = name.len().min(out.len());
            out[..n].copy_from_slice(name.as_bytes());
            got_len = Some(n);
            true
        } else {
            defmt::debug!("jdy at: {}", s);
            false
        }
    })
    .await;

    if ok {
        got_len
    } else {
        None
    }
}

async fn set_audio_name(
    tx: &mut UarteTx<'_>,
    rx: &mut UarteRxWithIdle<'_>,
    line: &mut LineBuf,
) -> bool {
    let mut cmd = [0u8; 40];
    let prefix = b"AT+NAMA";
    let name = AUDIO_BT_NAME.as_bytes();
    if prefix.len() + name.len() > cmd.len() {
        return false;
    }
    cmd[..prefix.len()].copy_from_slice(prefix);
    cmd[prefix.len()..prefix.len() + name.len()].copy_from_slice(name);
    let cmd_len = prefix.len() + name.len();
    let cmd_str = core::str::from_utf8(&cmd[..cmd_len]).unwrap_or("");

    line.clear();
    flush_rx(rx).await;
    line.clear();

    log_line::line("BT name: updating...");
    if send_at(tx, cmd_str).await.is_err() {
        warn!("AT+NAMA set send failed");
        log_line::line("BT name: set send failed");
        return false;
    }

    wait_lines(rx, line, JDY_AT_TIMEOUT_MS, |s| {
        defmt::debug!("jdy at: {}", s);
        is_ok_line(s) || parse_nama_value(s).is_some()
    })
    .await
}

async fn ensure_audio_bt_name(tx: &mut UarteTx<'_>, rx: &mut UarteRxWithIdle<'_>) {
    Timer::after(Duration::from_millis(150)).await;

    let mut line = LineBuf::new();
    let mut name_buf = [0u8; 24];

    match query_audio_name(tx, rx, &mut line, &mut name_buf).await {
        Some(n) => {
            let current = core::str::from_utf8(&name_buf[..n]).unwrap_or("");
            if current == AUDIO_BT_NAME {
                log_line::line("BT name: OK (BiuSpeaker)");
                return;
            }
            log_line::line("BT name: mismatch, will update");
        }
        None => {
            warn!("BT name query failed, trying set anyway");
            log_line::line("BT name: query failed, try set");
        }
    }

    if set_audio_name(tx, rx, &mut line).await {
        log_line::line("BT name: set BiuSpeaker");
        Timer::after(Duration::from_millis(100)).await;
        if let Some(n) = query_audio_name(tx, rx, &mut line, &mut name_buf).await {
            let current = core::str::from_utf8(&name_buf[..n]).unwrap_or("");
            if current == AUDIO_BT_NAME {
                log_line::line("BT name: verified BiuSpeaker");
            } else {
                warn!("BT name after set still wrong");
                log_line::line("BT name: verify failed");
            }
        }
    } else {
        warn!("BT name set failed or no OK");
        log_line::line("BT name: set failed");
    }
}

#[embassy_executor::task]
async fn uart_rx_task(
    uarte: Peri<'static, embassy_nrf::peripherals::UARTE0>,
    timer: Peri<'static, embassy_nrf::peripherals::TIMER0>,
    ppi_ch1: Peri<'static, embassy_nrf::peripherals::PPI_CH0>,
    ppi_ch2: Peri<'static, embassy_nrf::peripherals::PPI_CH1>,
    rxd: Peri<'static, embassy_nrf::peripherals::P0_06>,
    txd: Peri<'static, embassy_nrf::peripherals::P0_02>,
) {
    let mut config = uarte::Config::default();
    config.baudrate = Baudrate::BAUD9600;
    config.parity = Parity::EXCLUDED;

    let uart = Uarte::new(uarte, rxd, txd, UarteIrqs, config);
    let (mut tx, mut rx) = uart.split_with_idle(timer, ppi_ch1, ppi_ch2);

    log_line::line("JDY UART ready");
    ensure_audio_bt_name(&mut tx, &mut rx).await;
    let _ = send_at(&mut tx, "AT+ENLOG1").await;
    Timer::after(Duration::from_millis(50)).await;
    flush_rx(&mut rx).await;
    // JDY-68A: AT+PSTAT has no reply (unlike some JDY-67 docs). Keep listening for +PLAY/+PAUSE.
    log_line::line("play detect: wait +PLAY/+PAUSE (PSTAT N/A)");

    let mut line = LineBuf::new();
    let mut chunk = [0u8; 64];

    loop {
        match select(
            rx.read_until_idle(&mut chunk),
            Timer::after(Duration::from_millis(JDY_STAT_POLL_MS)),
        )
        .await
        {
            Either::First(Ok(n)) if n > 0 => {
                for &b in &chunk[..n] {
                    if let Some(s) = line.push(b) {
                        log_uart_raw(s);
                        if let Some(ev) = parse_jdy_line(s) {
                            publish(ev, EventSource::Uart).await;
                        }
                    }
                }
            }
            Either::First(Ok(_)) => {}
            Either::First(Err(e)) => {
                warn!("uart rx err: {:?}", e);
                Timer::after(Duration::from_millis(20)).await;
            }
            Either::Second(()) => {
                // Link status only
                let _ = send_at(&mut tx, "AT+STAT").await;
            }
        }
    }
}

async fn read_stat_stable(stat: &Input<'_>) -> bool {
    let mut highs = 0u8;
    let mut lows = 0u8;
    for _ in 0..STAT_STABLE_SAMPLES {
        if stat.is_high() {
            highs = highs.saturating_add(1);
        } else {
            lows = lows.saturating_add(1);
        }
        Timer::after(Duration::from_millis(STAT_SAMPLE_MS)).await;
    }
    highs > lows
}

#[embassy_executor::task]
async fn stat_task(pin: Peri<'static, embassy_nrf::peripherals::P0_17>) {
    // NOTE: On some dual-mode modules STAT=BLE only; audio uses ASTAT / UART.
    let stat = Input::new(pin, Pull::Down);
    log_line::line("STAT pin: wait (BLE pin, may stay low for audio)");
    Timer::after(Duration::from_millis(STAT_STARTUP_DELAY_MS)).await;

    let mut connected = read_stat_stable(&stat).await;
    if connected {
        publish(AudioEvent::Connected, EventSource::Stat).await;
    } else {
        publish(AudioEvent::Disconnected, EventSource::Stat).await;
    }

    loop {
        let now = stat.is_high();
        if now != connected {
            Timer::after(Duration::from_millis(BUTTON_DEBOUNCE_MS)).await;
            let confirmed = read_stat_stable(&stat).await;
            if confirmed != connected {
                connected = confirmed;
                if connected {
                    publish(AudioEvent::Connected, EventSource::Stat).await;
                } else {
                    publish(AudioEvent::Disconnected, EventSource::Stat).await;
                }
            }
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}

pub fn spawn(
    spawner: &Spawner,
    uarte: Peri<'static, embassy_nrf::peripherals::UARTE0>,
    timer: Peri<'static, embassy_nrf::peripherals::TIMER0>,
    ppi_ch1: Peri<'static, embassy_nrf::peripherals::PPI_CH0>,
    ppi_ch2: Peri<'static, embassy_nrf::peripherals::PPI_CH1>,
    rxd: Peri<'static, embassy_nrf::peripherals::P0_06>,
    txd: Peri<'static, embassy_nrf::peripherals::P0_02>,
    stat: Peri<'static, embassy_nrf::peripherals::P0_17>,
) {
    spawner
        .spawn(uart_rx_task(uarte, timer, ppi_ch1, ppi_ch2, rxd, txd).unwrap());
    spawner.spawn(stat_task(stat).unwrap());
}
