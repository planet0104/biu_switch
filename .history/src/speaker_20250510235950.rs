use stm32f4xx_hal::gpio::{DefaultMode, Output, PA5};

pub struct Speaker{
    floating_pin: Option<PA5<DefaultMode>>,
    output_pin: Option<PA5<Output>>
}

impl Speaker{
    
    pub fn new(pin: PA5<DefaultMode>) -> Speaker{
        Self { floating_pin: Some(pin), output_pin: None }
    }

    // 关闭音箱一定的时间(继电器io连接低电平，触发继电器)
    pub fn turnoff(&mut self){
        if let Some(pin) = self.floating_pin.take(){
            let mut pin_output = pin.into_push_pull_output();
            pin_output.set_low();
            self.output_pin.replace(pin_output);
        }
    }

    // 打开音箱(继电器io断开低电平)
    pub fn turnon(&mut self){
        if let Some(pin) = self.output_pin.take(){
            let floating_pin = pin.into_floating_input();
            self.floating_pin.replace(floating_pin);
        }
    }
}