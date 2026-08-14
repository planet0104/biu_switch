//! 唤醒按钮：运行中按下视为活动（取消自动关机倒计时）。
//! SYSTEM OFF 唤醒由 `power` 配置 P0.09 SENSE 完成（冷启动）。

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::Peri;
use embassy_time::{Duration, Timer};

use crate::events::{ShutdownCtrl, SHUTDOWN_CTRL};
use crate::log_line;
use crate::pins::BUTTON_DEBOUNCE_MS;

#[embassy_executor::task]
async fn button_task(pin: Peri<'static, embassy_nrf::peripherals::P0_09>) {
    let button = Input::new(pin, Pull::Up);
    let mut last_pressed = false;

    loop {
        let pressed = button.is_low();
        if pressed && !last_pressed {
            Timer::after(Duration::from_millis(BUTTON_DEBOUNCE_MS)).await;
            if button.is_low() {
                log_line::line("button: cancel auto-shutdown");
                let _ = SHUTDOWN_CTRL.try_send(ShutdownCtrl::Cancel);
                last_pressed = true;
            }
        } else if !pressed {
            last_pressed = false;
        }
        Timer::after(Duration::from_millis(20)).await;
    }
}

pub fn spawn(spawner: &Spawner, pin: Peri<'static, embassy_nrf::peripherals::P0_09>) {
    spawner.spawn(button_task(pin).unwrap());
}
