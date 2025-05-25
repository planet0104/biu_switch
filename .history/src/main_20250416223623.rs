#![no_main]
#![no_std]

use cortex_m_semihosting::hprintln;
use panic_semihosting as _;
use stm32f0xx_hal::{
    self as hal,
    adc::Adc,
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
    let mut delay_poweroff_minutes: i32 = 10;
    let mut elapsed_millis: i32 = 0;

    if let (Some(mut p), Some(cp)) = (pac::Peripherals::take(), Peripherals::take()) {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);

        let gpioa = p.GPIOA.split(&mut rcc);
        
        let mut gpio_switch_speaker = cortex_m::interrupt::free(move |cs| gpioa.pa5.into_push_pull_output(cs));
        let mut gpio_beep = cortex_m::interrupt::free(move |cs| gpioa.pa6.into_push_pull_output(cs));
        let mut gpio_button = cortex_m::interrupt::free(move |cs| gpioa.pa7.into_pull_up_input(cs));
        let mut speaker_volt = cortex_m::interrupt::free(move |cs| gpioa.pa0.into_analog(cs));
        let mut adc = Adc::new(p.ADC, &mut rcc);
        
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
                if delay_poweroff_minutes >= 60{
                    //超过60切换为1分钟
                    delay_poweroff_minutes = 1;
                }
                //如果是1分钟，按下按钮切换为10分钟
                else if delay_poweroff_minutes == 1{
                    delay_poweroff_minutes = 10;
                }else{
                    delay_poweroff_minutes += 10;
                }
                //计算有几个10分钟，不足1个10分钟说明是1分钟
                let ten_sec = delay_poweroff_minutes/10;
                if ten_sec == 0{
                    beep(&mut gpio_beep, &mut delay, 1, Some(500u16));
                }else{
                    beep(&mut gpio_beep, &mut delay, ten_sec as u8, None);
                }
                let elapsed_seconds = elapsed_millis as i32/1000;
                hprintln!("{}s/{}", elapsed_seconds, delay_poweroff_minutes);
            }
            delay.delay_ms(10_u16);
            elapsed_millis += 10;

            let time: u16 = if let Ok(val) = adc.read(&mut speaker_volt) as Result<u16, _> {
                /* shift the value right by 3, same as divide by 8, reduces
                the 0-4095 range into something approximating 1-512 */
                (val >> 3) + 1
            } else {
                1000
            };

            let elapsed_minutes = elapsed_millis/1000/60;
            if elapsed_minutes >= delay_poweroff_minutes{
                hprintln!("power off!! secs:{}", elapsed_millis/1000);
                power_off(&mut gpio_beep, &mut delay);
            }
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

fn beep(pin: &mut PA6<Output<PushPull>>, delay:&mut Delay, times: u8, duration: Option<u16>) {
    let dur = duration.unwrap_or(70u16);
    for _ in 0..times{
        let _ = pin.set_high();
        delay.delay_ms(dur);
        let _ = pin.set_low();
        delay.delay_ms(dur);
    }
}

fn power_off(beep_pin: &mut PA6<Output<PushPull>>, delay:&mut Delay){
    //测试，一长两短
    beep(beep_pin, delay, 1, Some(500u16));
    beep(beep_pin, delay, 1, None);
    beep(beep_pin, delay, 1, None);
    loop{
        delay.delay_ms(1000u16);
    }
}