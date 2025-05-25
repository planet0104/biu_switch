#![deny(unsafe_code)]
#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use cortex_m_semihosting::hprintln;
use panic_semihosting as _;

use cortex_m_rt::entry;
use stm32f4xx_hal::{
    self as hal, gpio::{Input, Output, DefaultMode, Pin, PA5}, i2s::{
        stm32_i2s_v12x::{
            marker::{Data16Channel16, Philips},
            transfer::{I2sTransfer, I2sTransferConfig},
        },
        I2s,
    }, pac::TIM5, timer::Delay
};

use crate::hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(_cp)) = (
        pac::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        //A5继电器(接地),A6电源(接地),A7切换蓝牙(接地)

        // Set up the LED. On the Mini-F4 it's connected to pin PC13.
        let gpioc = dp.GPIOC.split();
        let gpiob = dp.GPIOB.split();
        let gpioa = dp.GPIOA.split();
        // let mut relay_control = gpioa.pa5.into_floating_input();
        let mut power_control = gpioa.pa6.into_floating_input();
        // let mut bluetooth_switch = gpioa.pa7.into_floating_input();
        
        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = rcc
            .cfgr
            .use_hse(25.MHz())
            .sysclk(48.MHz())
            .i2s_clk(61440.kHz())
            .freeze();

        // I2S pins: (WS, CK, MCLK, SD) for I2S2
        let i2s2_pins = (
            gpiob.pb12, //WS
            gpiob.pb13, //CK
            gpioc.pc6,  //MCK
            gpiob.pb15, //SD
        );
        let i2s2 = I2s::new(dp.SPI2, i2s2_pins, &clocks);

        let transfer_config = I2sTransferConfig::new_master()
            .receive()
            .standard(Philips)
            .data_format(Data16Channel16)
            .master_clock(true)
            .request_frequency(16_000);

        let mut transfer = I2sTransfer::new(i2s2, transfer_config);
        transfer.begin();

        // Create a delay abstraction based on general-pupose 32-bit timer TIM5
        let mut delay = dp.TIM5.delay_us(&clocks);

        // power_control.set_high();
        // delay.delay_ms(1500);
        // power_control.set_low();

        // turnoff_speaker(&relay_control, &delay, 100);

        //读取1秒钟缓冲区
        let mut buf = [0i16; 16000];

        loop {
            let mut buf_iter = buf.iter_mut().peekable();

            //阻塞读取1秒钟的声音数据
            let _ = transfer.read_while(|s: (i16, i16)| {
                if let Some(b) = buf_iter.next() {
                    *b = s.0
                }
                buf_iter.peek().is_some()
            });

            //计算平均音量
            let avg = buf
                .iter()
                .map(|&x| x.abs() as i32) // 转成i32防止溢出
                .sum::<i32>()
                / buf.len() as i32;
            // avg/10 没有声音时=3，播放音乐时，大于等于5!!
            hprintln!("{}", avg / 10);
        }
    }

    loop {}
}

// 关闭麦克风指定v(继电器io连接低电平，触发继电器)
// fn turnoff_speaker(pin: &PA5<DefaultMode>, delay:&Delay<TIM5, 1000000>, duration_ms: u32){
//     let _ = pin.set_low();
//     // delay.delay_ms(100u16);
//     // let _ = pin.set_low();
// }

// fn beep(pin: &mut PA6<Output<PushPull>>, delay:&mut Delay, times: u8, duration: Option<u16>) {
//     let dur = duration.unwrap_or(70u16);
//     for _ in 0..times{
//         let _ = pin.set_high();
//         delay.delay_ms(dur);
//         let _ = pin.set_low();
//         delay.delay_ms(dur);
//     }
// }

// fn power_off(beep_pin: &mut PA6<Output<PushPull>>, delay:&mut Delay){
//     //测试，一长两短
//     beep(beep_pin, delay, 1, Some(500u16));
//     beep(beep_pin, delay, 1, None);
//     beep(beep_pin, delay, 1, None);
//     loop{
//         delay.delay_ms(1000u16);
//     }
// }

// loop {
//             // On for 1s, off for 3s.
//             led.set_high();
//             // Use `embedded_hal_02::DelayMs` trait
//             delay.delay_ms(1000);
//             led.set_low();
//             // or use `fugit::ExtU32` trait
//             delay.delay(3.secs());
//             hprintln!("-");
//         }
