//! 电池电压采样与低电量标志。

use core::sync::atomic::Ordering;

use embassy_executor::Spawner;
use embassy_nrf::saadc::{ChannelConfig, Config, Saadc};
use embassy_nrf::{bind_interrupts, saadc, Peri};

use crate::board_led::LOW_BATTERY;

bind_interrupts!(struct SaadcIrqs {
    SAADC => saadc::InterruptHandler;
});

const DIVIDER_RATIO: u32 = 3;
const BAT_LOW_MV: u32 = 3500;
/// 低于此值视为未接电池/分压未就绪，不触发低电量快闪（仅 USB 供电时 P0.04 常读数偏低）。
const BAT_PRESENT_MIN_MV: u32 = 500;
const LOW_BAT_CONFIRM_SAMPLES: u8 = 3;

fn sample_to_mv(raw: i16) -> u32 {
    let raw = raw.max(0) as u32;
    let adc_mv = raw * 3600 / 4096;
    adc_mv * DIVIDER_RATIO
}

#[embassy_executor::task]
async fn battery_task(mut saadc: Saadc<'static, 1>) {
    let mut low_streak: u8 = 0;

    loop {
        let mut buf = [0i16; 1];
        saadc.sample(&mut buf).await;
        let mv = sample_to_mv(buf[0]);

        if mv < BAT_PRESENT_MIN_MV {
            low_streak = 0;
            LOW_BATTERY.store(false, Ordering::Relaxed);
        } else if mv < BAT_LOW_MV {
            low_streak = low_streak.saturating_add(1);
            LOW_BATTERY.store(low_streak >= LOW_BAT_CONFIRM_SAMPLES, Ordering::Relaxed);
        } else {
            low_streak = 0;
            LOW_BATTERY.store(false, Ordering::Relaxed);
        }

        defmt::debug!("battery {} mV low={}", mv, LOW_BATTERY.load(Ordering::Relaxed));
        embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
    }
}

pub fn spawn(
    spawner: &Spawner,
    saadc: Peri<'static, embassy_nrf::peripherals::SAADC>,
    pin: Peri<'static, embassy_nrf::peripherals::P0_04>,
) {
    let config = Config::default();
    let channel = ChannelConfig::single_ended(pin);
    let saadc = Saadc::new(saadc, SaadcIrqs, config, [channel]);
    spawner.spawn(battery_task(saadc).unwrap());
}
