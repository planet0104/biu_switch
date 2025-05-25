//! Blinks an LED

#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use panic_semihosting as _;

use stm32f4xx_hal as hal;
use cortex_m_semihosting::hprintln;
use crate::hal::{pac, prelude::*};
use cortex_m::{delay, peripheral::Peripherals};
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let cp = Peripherals::take().unwrap();

    let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);

    // Get delay provider
    let mut delay = Delay::new(cp.SYST, &rcc);

    let gpioc = p.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output();

    loop {
        for _ in 0..10_000 {
            led.set_high();
        }
        for _ in 0..10_000 {
            led.set_low();
        }
    }
}