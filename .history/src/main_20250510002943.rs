#![deny(unsafe_code)]
#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use cortex_m_semihosting::hprintln;
use panic_semihosting as _;

use cortex_m_rt::entry;
use stm32f4xx_hal::{self as hal, i2s::{stm32_i2s_v12x::{driver::{ClockPolarity, DataFormat, I2sDriver, I2sDriverConfig}, marker::{Master, Philips, Receive}, transfer::I2sTransfer}, I2s}, pac::SPI2};

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
        let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(48.MHz()).freeze();

        // I2S pins: (WS, CK, MCLK, SD) for I2S2
        let i2s2_pins = (
            gpiob.pb12, //WS
            gpiob.pb13, //CK
            gpioc.pc6,  //MCK
            gpiob.pb15, //SD
        );
        let i2s2 = I2s::new(dp.SPI2, i2s2_pins, &clocks);
        let i2s2_config = I2sDriverConfig::new_master()
            .receive()
            .standard(Philips)
            .data_format(DataFormat::Data16Channel16)
            .master_clock(true)
            // .clock_polarity(ClockPolarity::IdleLow)
            .request_frequency(16_000);
        let mut i2s2_driver = I2s2Driver::new(i2s2, i2s2_config);
        hprintln!("actual sample rate is {}", i2s2_driver.sample_rate());
        // i2s2_driver.set_rx_interrupt(true);
        // i2s2_driver.set_error_interrupt(true);

        i2s2_driver.enable();

         let mut transfer = I2sTransfer::new(i2s2, transfer_config);

        // let status = i2s2_driver.status();

        // Create a delay abstraction based on general-pupose 32-bit timer TIM5
        let mut delay = dp.TIM5.delay_us(&clocks);
        
        let mut buffer = [0i16; 128];

        loop {
            let status = i2s2_driver.status();
            
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