#![no_main]
#![no_std]

use cortex_m_semihosting::hprintln;
use embedded_hal::digital::OutputPin;
use panic_semihosting as _;

use stm32f0xx_hal::{self as hal, gpio::gpiob, pac::GPIOA};

use crate::hal::{delay::Delay, pac, prelude::*};

use cortex_m::peripheral::Peripherals;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    if let (Some(mut p), Some(cp)) = (pac::Peripherals::take(), Peripherals::take()) {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);


        let gpioa = p.GPIOA.split(&mut rcc);
        
        let mut gpio_switch_speaker = cortex_m::interrupt::free(move |cs| gpioa.pa65.into_push_pull_output(cs));
        let mut gpio_beep = cortex_m::interrupt::free(move |cs| gpioa.pa6.into_push_pull_output(cs));
        

        // Get delay provider
        let mut delay = Delay::new(cp.SYST, &rcc);

        loop {
            gpio_beep.set_high();
            gpio_beep.toggle().ok();
            delay.delay_ms(1_000_u16);
            hprintln!("-");
        }
    }

    loop {
        continue;
    }
}


fn beep(gpio: &dyn OutputPin, times: u8){

}