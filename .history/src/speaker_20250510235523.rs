use stm32f4xx_hal::gpio::{DefaultMode, PA5};

struct Speaker{
    floating_pin: Option<PA5<DefaultMode>>,
    output_pin: Option<PA5<DefaultMode>>
}

impl Speaker{
    
    // 关闭音箱一定的时间(继电器io连接低电平，触发继电器)
    fn turnoff(pin: PA5<DefaultMode>) -> PA5<DefaultMode>{
        let mut pin_output = pin.into_push_pull_output();
        pin_output.set_low();
        //切换回阻塞模式
        pin_output.into_floating_input()
    }

    // 关闭音箱一定的时间(继电器io连接低电平，触发继电器)
    fn turnon(pin: PA5<DefaultMode>) -> PA5<DefaultMode>{
        let mut pin_output = pin.into_push_pull_output();
        pin_output.set_low();
        //切换回阻塞模式
        pin_output.into_floating_input()
    }
}