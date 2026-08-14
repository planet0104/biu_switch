//! USB CDC：明文 ASCII 日志（普通串口助手可直接看）+ `BOOT` 进 UF2 Bootloader。
//!
//! 与 defmt 二进制帧不同，本模块输出可读文本，避免 SSCOM 等工具乱码。

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_executor::Spawner;
use embassy_futures::join::join3;
use embassy_nrf::pac;
use embassy_nrf::usb::vbus_detect::HardwareVbusDetect;
use embassy_nrf::usb::Driver;
use embassy_nrf::{bind_interrupts, usb, Peri};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, Receiver, Sender, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};
use static_cell::{ConstStaticCell, StaticCell};

use crate::bootloader;

bind_interrupts!(struct UsbIrqs {
    USBD => usb::InterruptHandler<embassy_nrf::peripherals::USBD>;
    CLOCK_POWER => usb::vbus_detect::InterruptHandler;
});

static CONFIG_DESCRIPTOR_BUF: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0u8; 256]);
static BOS_DESCRIPTOR_BUF: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0u8; 256]);
static MSOS_DESCRIPTOR_BUF: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0u8; 256]);
static CONTROL_BUF: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0u8; 256]);
static CDC_STATE: StaticCell<State> = StaticCell::new();

const LOG_CAP: usize = 96;
static LOG_CH: Channel<CriticalSectionRawMutex, LogMsg, 24> = Channel::new();

/// defmt 仍需要 global logger；USB 明文日志走 [`write_line`]，此处丢弃 defmt 二进制帧。
#[defmt::global_logger]
struct DefmtNop;

static DEFMT_TAKEN: AtomicBool = AtomicBool::new(false);

unsafe impl defmt::Logger for DefmtNop {
    fn acquire() {
        while DEFMT_TAKEN
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {}
    }

    unsafe fn flush() {}

    unsafe fn release() {
        DEFMT_TAKEN.store(false, Ordering::Release);
    }

    unsafe fn write(_bytes: &[u8]) {}
}

#[derive(Clone, Copy)]
struct LogMsg {
    buf: [u8; LOG_CAP],
    len: u8,
}

impl LogMsg {
    const fn empty() -> Self {
        Self {
            buf: [0; LOG_CAP],
            len: 0,
        }
    }

    fn from_str(s: &str) -> Self {
        let mut m = Self::empty();
        let n = s.len().min(LOG_CAP);
        m.buf[..n].copy_from_slice(&s.as_bytes()[..n]);
        m.len = n as u8;
        m
    }

    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }
}

/// 排队一行 ASCII 日志（自动补 `\r\n`）。通道满则丢弃。
pub fn write_line(msg: &str) {
    let _ = LOG_CH.try_send(LogMsg::from_str(msg));
}

struct LineBuf {
    data: [u8; 16],
    len: usize,
}

impl LineBuf {
    const fn new() -> Self {
        Self {
            data: [0; 16],
            len: 0,
        }
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
                self.len = 0;
                None
            }
        }
    }
}

fn is_boot_cmd(line: &str) -> bool {
    line.trim().eq_ignore_ascii_case("BOOT")
}

async fn handle_boot_cmd() {
    write_line("BOOT cmd -> UF2 bootloader");
    Timer::after(Duration::from_millis(150)).await;
    bootloader::enter_uf2_bootloader();
}

async fn write_all<'d, D: embassy_usb::driver::Driver<'d>>(
    sender: &mut Sender<'d, D>,
    data: &[u8],
) -> Result<(), EndpointError> {
    let mut offset = 0;
    while offset < data.len() {
        let end = (offset + 64).min(data.len());
        sender.write_packet(&data[offset..end]).await?;
        offset = end;
    }
    Ok(())
}

async fn send_line<'d, D: embassy_usb::driver::Driver<'d>>(
    sender: &mut Sender<'d, D>,
    msg: &LogMsg,
) -> Result<(), EndpointError> {
    write_all(sender, msg.as_bytes()).await?;
    write_all(sender, b"\r\n").await
}

async fn text_logger<'d, D: embassy_usb::driver::Driver<'d>>(mut sender: Sender<'d, D>) {
    loop {
        sender.wait_connection().await;
        let _ = send_line(&mut sender, &LogMsg::from_str("usb cdc ready")).await;

        loop {
            let msg = LOG_CH.receive().await;
            match send_line(&mut sender, &msg).await {
                Ok(()) => {}
                Err(EndpointError::Disabled) => break,
                Err(EndpointError::BufferOverflow) => break,
            }
        }
    }
}

async fn cmd_reader<'d, D: embassy_usb::driver::Driver<'d>>(mut receiver: Receiver<'d, D>) {
    let mut packet = [0u8; 64];
    let mut line = LineBuf::new();

    loop {
        receiver.wait_connection().await;
        loop {
            match receiver.read_packet(&mut packet).await {
                Ok(0) => {}
                Ok(n) => {
                    for &byte in &packet[..n] {
                        if let Some(line_str) = line.push(byte) {
                            if is_boot_cmd(line_str) {
                                handle_boot_cmd().await;
                            }
                        }
                    }
                }
                Err(EndpointError::Disabled) => break,
                Err(EndpointError::BufferOverflow) => unreachable!(),
            }
        }
    }
}

pub fn enable_hfclk() {
    pac::CLOCK.tasks_hfclkstart().write_value(1);
    while pac::CLOCK.events_hfclkstarted().read() != 1 {}
}

#[embassy_executor::task]
async fn usb_cdc_task(usbd: Peri<'static, embassy_nrf::peripherals::USBD>) {
    let driver = Driver::new(usbd, UsbIrqs, HardwareVbusDetect::new(UsbIrqs));
    let mut config = Config::new(0x239a, 0x0029);
    config.manufacturer = Some("biu_switch");
    config.product = Some("biu_switch");
    config.serial_number = Some("cdc1");
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.composite_with_iads = true;
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    let packet_size = config.max_packet_size_0 as u16;

    let state = CDC_STATE.init(State::new());
    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESCRIPTOR_BUF.take(),
        BOS_DESCRIPTOR_BUF.take(),
        MSOS_DESCRIPTOR_BUF.take(),
        CONTROL_BUF.take(),
    );

    let class = CdcAcmClass::new(&mut builder, state, packet_size);
    let mut usb = builder.build();
    let (sender, receiver) = class.split();

    join3(usb.run(), text_logger(sender), cmd_reader(receiver)).await;
}

pub fn spawn(spawner: &Spawner, usbd: Peri<'static, embassy_nrf::peripherals::USBD>) {
    spawner.spawn(usb_cdc_task(usbd).unwrap());
}
