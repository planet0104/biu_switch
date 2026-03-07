use stm32f4xx_hal::gpio::{DefaultMode, Output, PA5};


// 音箱开关控制器
// 硬件连接线：固态继电器 IN1 接PA5，IN2接GND

pub struct Speaker{
    output_pin: PA5<Output>
}

impl Speaker{
    
    pub fn new(pin: PA5<DefaultMode>) -> Speaker{
        //初始化时直接设置为输出模式
        let mut output_pin = pin.into_push_pull_output();
        //初始化为高电平（即继电器关断状态）
        output_pin.set_high();
        Self { output_pin }
    }

    // 关闭音箱(继电器动作)
    // 逻辑：输出低电平
    pub fn turnoff(&mut self){
        //输出低电平
        self.output_pin.set_low();
    }

    // 打开音箱(继电器释放)
    // 逻辑：输出高电平
    pub fn turnon(&mut self){
        // 直接设置为高电平
        self.output_pin.set_high();
    }
}