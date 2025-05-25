#![no_main]
#![no_std]

use cortex_m_semihosting::hprintln;
use panic_semihosting as _;
use stm32f0xx_hal::{
    self as hal,
    gpio::{
        gpioa::{PA5, PA6},
        gpiob,
        Output,
        PushPull
    }
};

use crate::hal::{delay::Delay, pac, prelude::*};
use cortex_m::{delay, peripheral::Peripherals};
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {

    let mut speaker_opend = false;

    // 默认10分钟关机
    let mut delay_poweroff_minutes = 10;
    let mut elapsed_millis = 0;

    if let (Some(mut p), Some(cp)) = (pac::Peripherals::take(), Peripherals::take()) {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);

        let gpioa = p.GPIOA.split(&mut rcc);
        
        let mut gpio_switch_speaker = cortex_m::interrupt::free(move |cs| gpioa.pa5.into_push_pull_output(cs));
        let mut gpio_beep = cortex_m::interrupt::free(move |cs| gpioa.pa6.into_push_pull_output(cs));
        let mut gpio_button = cortex_m::interrupt::free(move |cs| gpioa.pa7.into_pull_up_input(cs));

        // Get delay provider
        let mut delay = Delay::new(cp.SYST, &rcc);

        // hprintln!("Flash len:{}", p.FLASH.len());

        loop {
            // toggle_speaker_switch(&mut gpio_switch_speaker, &mut delay, &mut speaker_opend);
            // beep(&mut gpio_beep, &mut delay, 1);
            // delay.delay_ms(3_000_u16);
            // hprintln!("-");
            if let Ok(true) = gpio_button.is_low(){
                delay.delay_ms(100_u16);
                if delay_poweroff_minutes >= 30{
                    delay_poweroff_minutes = 0;
                }else{
                    delay_poweroff_minutes += 5;
                }
                let five_sec = delay_poweroff_minutes/5;
                beep(&mut gpio_beep, &mut delay, five_sec+1);

                let elapsed_seconds = elapsed_millis/1000;
                hprintln!("{}s/{}", elapsed_seconds, delay_poweroff_minutes);
            }
            delay.delay_ms(10_u16);
            elapsed_millis += 10;
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