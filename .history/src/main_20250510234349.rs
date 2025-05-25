#![deny(unsafe_code)]
#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

// use cortex_m_semihosting::hprintln;
use panic_semihosting as _;

use cortex_m_rt::entry;
use stm32f4xx_hal::{
    self as hal, gpio::{DefaultMode, Input, Output, Pin, PA5, PA6, PA7}, i2s::{
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
        let mut relay_control = gpioa.pa5.into_floating_input();
        let mut power_control = gpioa.pa6.into_floating_input();
        let mut bluetooth_switch = gpioa.pa7.into_floating_input();
        
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

        //等待开机完成后(3秒),切换到蓝牙模式
        delay.delay_ms(3000);
        bluetooth_switch = switch_to_bluetooth(bluetooth_switch, &mut delay);

        //读取1秒钟缓冲区
        let mut buf = [0i16; 16000];
        let mut no_sound_duration_seconds = 0;

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
            let avg = avg/10;
            // avg/10 没有声音时=3，播放音乐时，大于等于5!!
            // hprintln!("{}", avg / 10);
            if avg <= 4 {
                no_sound_duration_seconds += 1;
            }else{
                no_sound_duration_seconds = 0;
            }
            //1分钟后没有声音，关机
            if no_sound_duration_seconds > 60{
                power_control = power_off(power_control, &mut delay);
            }
        }
    }

    loop {}
}


//开机后，电源按钮引脚默认是floating_input，此时与负极断开
//关机时，将电源引脚转换成输出模式，并设置低电平，此时相当于按下了按钮，将会关机.
fn power_off(pin: PA6<DefaultMode>, delay:&mut Delay<TIM5, 1000000>) -> PA6<DefaultMode>{
    let mut pin_output = pin.into_push_pull_output();
    pin_output.set_low();
    delay.delay_ms(1500);
    //切换回阻塞模式
    pin_output.into_floating_input()
}

// 关闭麦克风一定的时间(继电器io连接低电平，触发继电器)
fn turnoff_speaker(pin: PA5<DefaultMode>, delay:&mut Delay<TIM5, 1000000>, duration_ms: u32) -> PA5<DefaultMode>{
    let mut pin_output = pin.into_push_pull_output();
    pin_output.set_low();
    delay.delay_ms(duration_ms);
    //切换回阻塞模式
    pin_output.into_floating_input()
}

// 切换到蓝牙模式(io连接低电平，触发蓝牙按钮)
fn switch_to_bluetooth(pin: PA7<DefaultMode>, delay:&mut Delay<TIM5, 1000000>) -> PA7<DefaultMode>{
    let mut pin_output = pin.into_push_pull_output();
    pin_output.set_low();
    delay.delay_ms(300);
    //切换回阻塞模式
    pin_output.into_floating_input()
}