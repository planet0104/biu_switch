//! 板载 P0.15 指示灯（PWM 呼吸：3 s 渐亮 + 3 s 渐灭）。
//!
//! 固定步数计时；亮度曲线为 sin²(π/2·t/T)，渐灭为同曲线倒放，全程对称且更均匀。

use core::sync::atomic::Ordering;

use embassy_executor::Spawner;
#[cfg(feature = "no-led")]
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::pwm::{Prescaler, SimplePwm};
use embassy_nrf::Peri;
use embassy_time::{Duration, Timer};
use portable_atomic::AtomicBool;

#[cfg(not(feature = "no-led"))]
use crate::pins::{
    LED_BREATHE_HALF_MS, LED_BREATHE_STEP_MS, LED_BREATHE_PWM_HZ, LED_LOW_BAT_FLASH_MS,
    LED_LOW_BAT_INTERVAL_SEC,
};

/// 定点 0..4096 表示 0.0..1.0
#[cfg(not(feature = "no-led"))]
const FP_ONE: u32 = 4096;

/// sin²(π/2 · i/63) · 4095，i = 0..63
#[cfg(not(feature = "no-led"))]
const SIN2_LUT: [u16; 64] = [
    0, 3, 10, 23, 41, 63, 91, 123, 161, 203, 249, 300, 356, 415, 479, 547, 618, 693, 771, 852,
    937, 1024, 1113, 1205, 1299, 1395, 1493, 1592, 1692, 1793, 1894, 1996, 2099, 2201, 2302,
    2403, 2503, 2602, 2700, 2796, 2890, 2982, 3071, 3158, 3243, 3324, 3402, 3477, 3548, 3616,
    3680, 3739, 3795, 3846, 3892, 3934, 3972, 4004, 4032, 4054, 4072, 4085, 4092, 4095,
];

pub static LOW_BATTERY: AtomicBool = AtomicBool::new(false);

#[cfg(not(feature = "no-led"))]
fn duty_for_brightness(max_duty: u16, brightness: u16) -> u16 {
    #[cfg(feature = "led-active-high")]
    {
        let _ = max_duty;
        brightness
    }
    #[cfg(not(feature = "led-active-high"))]
    {
        max_duty.saturating_sub(brightness)
    }
}

#[cfg(not(feature = "no-led"))]
fn set_brightness(pwm: &mut SimplePwm<'_>, max_duty: u16, brightness: u16) {
    pwm.set_duty(0, duty_for_brightness(max_duty, brightness));
}

/// sin² 半周期：t=0→T 对应 0→1；渐灭时 t 从 T 倒计到 0
#[cfg(not(feature = "no-led"))]
fn breathe_factor(step: u32, steps: u32, fade_in: bool) -> u32 {
    if steps == 0 {
        return if fade_in { 0 } else { FP_ONE };
    }
    let t = if fade_in { step } else { steps - step };
    if t == 0 {
        return 0;
    }
    if t >= steps {
        return FP_ONE;
    }
    let idx = (t as u64 * (SIN2_LUT.len() as u64 - 1) / steps as u64) as usize;
    SIN2_LUT[idx] as u32
}

#[cfg(not(feature = "no-led"))]
fn factor_to_brightness(max_duty: u16, factor: u32) -> u16 {
    ((max_duty as u64 * factor as u64 / FP_ONE as u64) as u16).min(max_duty)
}

#[cfg(not(feature = "no-led"))]
async fn breathe_half(pwm: &mut SimplePwm<'_>, max_duty: u16, fade_in: bool) {
    let steps = (LED_BREATHE_HALF_MS / LED_BREATHE_STEP_MS) as u32;
    let update = Duration::from_millis(LED_BREATHE_STEP_MS);

    for step in 0..=steps {
        let factor = breathe_factor(step, steps, fade_in);
        let brightness = factor_to_brightness(max_duty, factor);
        set_brightness(pwm, max_duty, brightness);
        if step < steps {
            Timer::after(update).await;
        }
    }
}

#[cfg(not(feature = "no-led"))]
#[embassy_executor::task]
async fn board_led_task(
    pwm1: Peri<'static, embassy_nrf::peripherals::PWM1>,
    pin: Peri<'static, embassy_nrf::peripherals::P0_15>,
) {
    let mut pwm = SimplePwm::new_1ch(pwm1, pin);
    pwm.set_prescaler(Prescaler::Div1);
    pwm.set_period(LED_BREATHE_PWM_HZ);
    pwm.enable();
    let max_duty = pwm.max_duty();
    set_brightness(&mut pwm, max_duty, 0);

    let mut low_bat_anchor = embassy_time::Instant::now();

    loop {
        if crate::power::SHUTTING_DOWN.load(Ordering::Acquire) {
            set_brightness(&mut pwm, max_duty, 0);
            pwm.disable();
            loop {
                Timer::after(Duration::from_secs(3600)).await;
            }
        }

        if LOW_BATTERY.load(Ordering::Relaxed) {
            if low_bat_anchor.elapsed() >= Duration::from_secs(LED_LOW_BAT_INTERVAL_SEC) {
                low_bat_anchor = embassy_time::Instant::now();
                set_brightness(&mut pwm, max_duty, max_duty);
                Timer::after(Duration::from_millis(LED_LOW_BAT_FLASH_MS)).await;
                set_brightness(&mut pwm, max_duty, 0);
            }
            Timer::after(Duration::from_millis(40)).await;
        } else {
            breathe_half(&mut pwm, max_duty, true).await;
            breathe_half(&mut pwm, max_duty, false).await;
        }
    }
}

#[cfg(feature = "no-led")]
fn led_off_gpio(led: &mut Output<'_>) {
    #[cfg(feature = "led-active-high")]
    led.set_low();
    #[cfg(not(feature = "led-active-high"))]
    led.set_high();
}

#[cfg(feature = "no-led")]
#[embassy_executor::task]
async fn board_led_off_task(mut led: Output<'static>) {
    led_off_gpio(&mut led);
    loop {
        Timer::after(Duration::from_secs(3600)).await;
    }
}

#[cfg(not(feature = "no-led"))]
pub fn spawn(
    spawner: &Spawner,
    pwm1: Peri<'static, embassy_nrf::peripherals::PWM1>,
    pin: Peri<'static, embassy_nrf::peripherals::P0_15>,
) {
    spawner.spawn(board_led_task(pwm1, pin).unwrap());
}

#[cfg(feature = "no-led")]
pub fn spawn(spawner: &Spawner, pin: Peri<'static, embassy_nrf::peripherals::P0_15>) {
    #[cfg(feature = "led-active-high")]
    let led = Output::new(pin, Level::Low, OutputDrive::Standard);
    #[cfg(not(feature = "led-active-high"))]
    let led = Output::new(pin, Level::High, OutputDrive::Standard);
    spawner.spawn(board_led_off_task(led).unwrap());
}
