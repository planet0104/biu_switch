#![no_main]
#![no_std]

use cortex_m_semihosting::hprintln;
use panic_semihosting as _;

use stm32f0xx_hal::{
    self as hal,
    flash::{FlashExt, LockedFlash},
    gpio::{gpioa::{PA5, PA6},
    gpiob,
    Output,
    PushPull},
    pac::GPIOA
};

use crate::hal::{delay::Delay, pac, prelude::*};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use cortex_m::{delay, peripheral::Peripherals};
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {

    let mut speaker_opend = false;

    if let (Some(mut p), Some(cp)) = (pac::Peripherals::take(), Peripherals::take()) {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);


        // // All examples use the first 16K of flash for the program so we use the first page after that
        const OFFSET_START: u32 = 16 * 1024;
        const OFFSET_END: u32 = OFFSET_START + 1024;
        // // Unlock flash before writing
        let mut unlocked_flash = p.FLASH.unlocked();

        hprintln!("hello offset:{}", OFFSET_END);
        let ret = NorFlash::erase(&mut unlocked_flash, OFFSET_START, OFFSET_END);
        hprintln!("ret:{ret:?}");

        let gpioa = p.GPIOA.split(&mut rcc);
        
        let mut gpio_switch_speaker = cortex_m::interrupt::free(move |cs| gpioa.pa5.into_push_pull_output(cs));
        let mut gpio_beep = cortex_m::interrupt::free(move |cs| gpioa.pa6.into_push_pull_output(cs));        

        // Get delay provider
        let mut delay = Delay::new(cp.SYST, &rcc);

        // hprintln!("Flash len:{}", p.FLASH.len());

        loop {
            toggle_speaker_switch(&mut gpio_switch_speaker, &mut delay, &mut speaker_opend);
            beep(&mut gpio_beep, &mut delay, 1);
            delay.delay_ms(3_000_u16);
            hprintln!("-");
        }
    }

    loop {
        continue;
    }
}

fn toggle_speaker_switch(pin: &mut PA5<Output<PushPull>>, delay:&mut Delay, speaker_opend:&mut bool){
    *speaker_opend = !*speaker_opend;
    let _ = pin.set_high();
    delay.delay_ms(100u16);
    let _ = pin.set_low();
}

fn beep(pin: &mut PA6<Output<PushPull>>, delay:&mut Delay, times: u8) {
    for _ in 0..times{
        let _ = pin.set_high();
        delay.delay_ms(70u16);
        let _ = pin.set_low();
        delay.delay_ms(70u16);
    }
}