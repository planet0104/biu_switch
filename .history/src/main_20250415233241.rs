#![no_main]
#![no_std]

use core::fmt::Error;

use cortex_m_semihosting::hprintln;
use embedded_hal::digital::OutputPin;
use panic_semihosting as _;

use stm32f0xx_hal::{self as hal, gpio::{gpioa::PA6, gpiob, Output, PushPull}, pac::GPIOA};

use crate::hal::{delay::Delay, pac, prelude::*};

use cortex_m::{delay, peripheral::Peripherals};
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {

    let mut speaker_opend = false;

    if let (Some(mut p), Some(cp)) = (pac::Peripherals::take(), Peripherals::take()) {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);


        let gpioa = p.GPIOA.split(&mut rcc);
        
        let mut gpio_switch_speaker = cortex_m::interrupt::free(move |cs| gpioa.pa5.into_push_pull_output(cs));
        let mut gpio_beep = cortex_m::interrupt::free(move |cs| gpioa.pa6.into_push_pull_output(cs));
        

        // Get delay provider
        let mut delay = Delay::new(cp.SYST, &rcc);

        loop {
            beep(&mut gpio_beep, &mut delay, 5);
            delay.delay_ms(3_000_u16);
            hprintln!("-");
        }
    }

    loop {
        continue;
    }
}

fn toggle_speaker_switch(speaker_opend:&mut bool){

    *speaker_opend = !*speaker_opend;
}

fn beep(pin: &mut PA6<Output<PushPull>>, delay:&mut Delay, times: u8) {
    for _ in 0..times{
        let _ = pin.set_high();
        delay.delay_ms(70u16);
        let _ = pin.set_low();
        delay.delay_ms(70u16);
    }
}