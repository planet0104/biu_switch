#![allow(clippy::empty_loop)]
#![allow(unsafe_code)]
#![no_main]
#![no_std]

use panic_semihosting as _;

use cortex_m_semihosting::hprintln;
use stm32f0xx_hal::{
    pac::Peripherals,
    prelude::*,
    spi::{Spi, *},
};

pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleHigh,
};

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let cp = cortex_m::Peripherals::take().unwrap();
    let mut delay = cp.SYST.delay(&clocks);

    let mut afio = dp.AFIO.constrain();
   
    loop {
        delay.delay_ms(3000u16);
        hprintln!("-");
    }
}