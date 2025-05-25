#![allow(clippy::empty_loop)]
#![allow(unsafe_code)]
#![no_main]
#![no_std]

use panic_semihosting as _;

use cortex_m_semihosting::hprintln;
use st7735::{Orientation, ST7735};
use stm32f1xx_hal::{
    pac::Peripherals,
    prelude::*,
    spi::{Spi, *},
};

pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleHigh,
};

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let cp = cortex_m::Peripherals::take().unwrap();
    let mut delay = cp.SYST.delay(&clocks);

    let mut afio = dp.AFIO.constrain();
    let mut gpioa = dp.GPIOA.split();
    let mut gpioc = dp.GPIOC.split();
    let mut gpiob = dp.GPIOB.split();

    // SPI1
    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

    //每次有效的数据传输之前先拉低CS，传输结束后再拉高CS
    let cs = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);

    let spi = Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        MODE,
        1.MHz(),
        clocks,
    );

    // PC9 connects to RST/RES on the LCD
    let rst = gpiob.pb0.into_push_pull_output(&mut gpiob.crl);

    // PB0 connects to RS/DC on the LCD
    let dc = gpiob.pb1.into_push_pull_output(&mut gpiob.crl);

    let mut disp = ST7735::new(spi, dc, Some(rst), true, false, 160, 80);

    // Initialize the display.
    disp.init(&mut delay).unwrap();
    // Set the orientation of the display
    disp.set_orientation(&Orientation::Landscape).unwrap();

    // disp.hard_reset(&mut delay);
    
    loop{
        disp.write_pixels(colors)
        disp.set_pixel(10, 10, 255).unwrap();
        delay.delay_ms(1000u32);
        disp.set_pixel(10, 10, 0).unwrap();
        delay.delay_ms(1000u32);
    }
    

    /* Create a style that specifies a color of RED. This will always use
    Rgb565 regardless if your board uses RGB or BGR. */
    // let style = PrimitiveStyleBuilder::new().fill_color(Rgb565::RED).build();

    /* Create a rectangle to fill the background. Make sure the second point
    has a width and height that matches your ST7735. */
    // let red_backdrop = Rectangle::new(Point::new(0, 0), Size::new(50, 50)).into_styled(style);
    // red_backdrop.draw(&mut disp).unwrap();

    loop {
        delay.delay_ms(3000u16);
        hprintln!("-");
    }
}