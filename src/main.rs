//! # biu_switch — nRF52840 + JDY-68A 蓝牙音箱固件
//!
//! 目标板：ProMicro nRF52840（兼容 Nice!Nano V2）

#![no_std]
#![no_main]

#[cfg(all(feature = "usb-log", feature = "defmt-rtt"))]
compile_error!("usb-log 与 defmt-rtt 不能同时启用");

use embassy_executor::Spawner;

#[cfg(not(feature = "usb-log"))]
use defmt_rtt as _;

#[cfg(feature = "usb-log")]
mod usb_log;

use {panic_probe as _};

mod audio_state;
mod auto_shutdown;
#[cfg(not(feature = "no-battery"))]
mod battery;
mod board_led;
mod bootloader;
mod button;
mod events;
mod jdy68a;
mod log_line;
mod pins;
mod power;

use events::{PowerCommand, POWER_CMDS};
use power::PowerManager;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    #[cfg(feature = "usb-log")]
    {
        usb_log::enable_hfclk();
        usb_log::spawn(&spawner, p.USBD);
    }

    log_line::line("biu_switch boot");

    #[cfg(not(feature = "no-led"))]
    board_led::spawn(&spawner, p.PWM1, p.P0_15);
    #[cfg(feature = "no-led")]
    board_led::spawn(&spawner, p.P0_15);

    #[cfg(not(feature = "no-battery"))]
    battery::spawn(&spawner, p.SAADC, p.P0_04);

    let mut power = PowerManager::new(p.P0_13, p.P0_08);
    power.power_on().await;

    audio_state::spawn(&spawner);
    auto_shutdown::spawn(&spawner);
    button::spawn(&spawner, p.P0_09);
    jdy68a::spawn(
        &spawner,
        p.UARTE0,
        p.TIMER0,
        p.PPI_CH0,
        p.PPI_CH1,
        p.P0_06,
        p.P0_02,
        p.P0_17,
    );

    log_line::line("speaker running");

    loop {
        match POWER_CMDS.receive().await {
            PowerCommand::PowerOff => {
                if power.is_powered_on() {
                    power.power_off_and_sleep().await;
                }
            }
        }
    }
}
