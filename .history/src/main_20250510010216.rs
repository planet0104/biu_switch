#![deny(unsafe_code)]
#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use cortex_m_semihosting::hprintln;
use panic_semihosting as _;

use cortex_m_rt::entry;
use stm32f4xx_hal::{self as hal, i2s::{stm32_i2s_v12x::{driver::{ClockPolarity, DataFormat, I2sDriver, I2sDriverConfig}, marker::{Data16Channel16, Master, Philips, Receive}, transfer::{I2sTransfer, I2sTransferConfig}}, I2s}, pac::SPI2};

use crate::hal::{pac, prelude::*};

type I2s2Driver = I2sDriver<I2s<SPI2>, Master, Receive, Philips>;

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(_cp)) = (
        pac::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        // Set up the LED. On the Mini-F4 it's connected to pin PC13.
        let gpioc = dp.GPIOC.split();
        let gpiob = dp.GPIOB.split();
        // let mut led = gpioc.pc13.into_push_pull_output();

        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz())
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
        
        //读取1秒钟缓冲区
        let mut buf = [0u8; 16000];
        let buf_len = buf.len();
        // peekable iterator
        let mut buf_iter = buf.iter_mut().peekable();

        loop {
            // take left channel data and convert it into 8 bit data (blocking)
            transfer.read_while(|s: (i16, i16)| {
                if let Some(b) = buf_iter.next() {
                    *b = (s.0 >> 8) as u8;
                }
                buf_iter.peek().is_some()
            });
            // 对u8数组需要先将值转换为有符号再计算
            let sum: i32 = buf.iter().map(|b| *b as i32 - 128).sum();
            let avg: i32 = sum.abs() / buf_len as i32;
            hprintln!("{}", avg);
        }
    }

    loop {}
}

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