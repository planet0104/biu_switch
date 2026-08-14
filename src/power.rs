//! 电源管理：P0.13（JDY VCC）+ P0.08（功放继电器）时序，以及 SYSTEM OFF。
//!
//! 开机：先蓝牙后功放；关机：先功放后蓝牙。关机前将 UART TX（P0.02）拉低，防止倒灌。

use core::sync::atomic::{AtomicBool, Ordering};

use cortex_m::interrupt;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::interrupt::InterruptExt;
use embassy_nrf::pac;
use embassy_nrf::pac::gpio::vals::{Dir, Input as InputMode, Pull as PacPull, Sense};
use embassy_nrf::pac::uarte::vals::Enable as UarteEnable;
use embassy_nrf::Peri;
use embassy_time::{Duration, Timer};

use crate::log_line;
use crate::pins::{
    AMP_OFF_SETTLE_MS, BOARD_LED_PIN, BUTTON_WAKE_PIN, JDY_BOOT_DELAY_MS, JDY_UART_TX_PIN,
};

/// 即将关机：板载 LED 任务应熄灭并停止呼吸。
pub static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub struct PowerManager {
    vcc: Output<'static>,
    amp: Output<'static>,
    powered: bool,
}

impl PowerManager {
    pub fn new(
        vcc_pin: Peri<'static, embassy_nrf::peripherals::P0_13>,
        amp_pin: Peri<'static, embassy_nrf::peripherals::P0_08>,
    ) -> Self {
        // 上电默认全关，避免未初始化时功放有电
        let vcc = Output::new(vcc_pin, Level::Low, OutputDrive::Standard);
        let amp = Output::new(amp_pin, Level::Low, OutputDrive::Standard);
        Self {
            vcc,
            amp,
            powered: false,
        }
    }

    pub fn is_powered_on(&self) -> bool {
        self.powered
    }

    /// Power on: JDY first, then amp.
    pub async fn power_on(&mut self) {
        log_line::line("power on: JDY then amp");
        tx_drive_low();
        self.amp.set_low();
        self.vcc.set_high();
        Timer::after(Duration::from_millis(JDY_BOOT_DELAY_MS)).await;
        self.amp.set_high();
        self.powered = true;
        log_line::line("power on: complete");
    }

    /// Power off then SYSTEM OFF (wake on P0.09).
    pub async fn power_off_and_sleep(&mut self) -> ! {
        log_line::line("power off sequence");
        defmt::flush();

        SHUTTING_DOWN.store(true, Ordering::Release);
        Timer::after(Duration::from_millis(30)).await;

        stop_uarte0();
        tx_drive_low();

        self.amp.set_low();
        Timer::after(Duration::from_millis(AMP_OFF_SETTLE_MS)).await;
        self.vcc.set_low();
        self.powered = false;

        enter_system_off()
    }
}

fn tx_drive_low() {
    pac::P0.pin_cnf(JDY_UART_TX_PIN as usize).write(|w| {
        w.set_dir(Dir::OUTPUT);
        w.set_input(InputMode::DISCONNECT);
        w.set_pull(PacPull::DISABLED);
        w.set_sense(Sense::DISABLED);
    });
    pac::P0.outclr().write(|w| w.0 = 1 << JDY_UART_TX_PIN);
}

fn stop_uarte0() {
    pac::UARTE0.tasks_stoptx().write_value(1);
    pac::UARTE0.tasks_stoprx().write_value(1);
    pac::UARTE0.enable().write(|w| w.set_enable(UarteEnable::DISABLED));
    embassy_nrf::interrupt::UARTE0.disable();
}

fn configure_button_wake() {
    pac::P0.pin_cnf(BUTTON_WAKE_PIN as usize).modify(|w| {
        w.set_dir(Dir::INPUT);
        w.set_input(InputMode::CONNECT);
        w.set_pull(PacPull::PULLUP);
        w.set_sense(Sense::LOW);
    });
}

fn board_led_gpio_off() {
    pac::PWM1.enable().write(|w| w.set_enable(false));
    pac::P0.pin_cnf(BOARD_LED_PIN as usize).write(|w| {
        w.set_dir(Dir::OUTPUT);
        w.set_input(InputMode::DISCONNECT);
        w.set_pull(PacPull::DISABLED);
    });
    #[cfg(feature = "led-active-high")]
    pac::P0.outclr().write(|w| w.0 = 1 << BOARD_LED_PIN);
    #[cfg(not(feature = "led-active-high"))]
    pac::P0.outset().write(|w| w.0 = 1 << BOARD_LED_PIN);
}

fn stop_time_driver_rtc1() {
    pac::RTC1.tasks_stop().write_value(1);
    pac::RTC1.intenclr().write(|w| w.0 = 0xFFFF_FFFF);
    embassy_nrf::interrupt::RTC1.disable();
}

fn stop_usb_and_hfclk() {
    pac::USBD.usbpullup().write(|w| w.set_connect(false));
    pac::USBD.enable().write(|w| w.set_enable(false));
    embassy_nrf::interrupt::USBD.disable();
    pac::CLOCK.tasks_hfclkstop().write_value(1);
}

fn stop_timers_and_pwm() {
    pac::TIMER0.tasks_stop().write_value(1);
    pac::PWM0.enable().write(|w| w.set_enable(false));
    pac::PWM1.enable().write(|w| w.set_enable(false));
}

fn prepare_for_system_off() {
    board_led_gpio_off();
    stop_timers_and_pwm();
    stop_time_driver_rtc1();
    stop_usb_and_hfclk();
    interrupt::disable();
    cortex_m::asm::dsb();
}

fn enter_system_off() -> ! {
    log_line::line("enter SYSTEM OFF");
    defmt::flush();
    configure_button_wake();
    prepare_for_system_off();
    embassy_nrf::power::set_system_off();
    loop {
        cortex_m::asm::wfi();
    }
}
