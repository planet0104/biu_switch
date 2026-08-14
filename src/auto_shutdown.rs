//! 自动关机：断开或暂停后 10 分钟无活动则 PowerOff。

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};

use crate::events::{PowerCommand, ShutdownCtrl, POWER_CMDS, SHUTDOWN_CTRL};
use crate::log_line;
use crate::pins::AUTO_OFF_SEC;

#[embassy_executor::task]
async fn auto_shutdown_task() {
    let mut armed = false;

    loop {
        if !armed {
            match SHUTDOWN_CTRL.receive().await {
                ShutdownCtrl::Arm => {
                    log_line::line("auto-shutdown: armed");
                    armed = true;
                }
                ShutdownCtrl::Cancel => {}
            }
            continue;
        }

        match select(
            SHUTDOWN_CTRL.receive(),
            Timer::after(Duration::from_secs(AUTO_OFF_SEC)),
        )
        .await
        {
            Either::First(ShutdownCtrl::Cancel) => {
                log_line::line("auto-shutdown: cancelled");
                armed = false;
            }
            Either::First(ShutdownCtrl::Arm) => {
                log_line::line("auto-shutdown: re-armed");
            }
            Either::Second(()) => {
                log_line::line("auto-shutdown: timeout -> power off");
                let _ = POWER_CMDS.try_send(PowerCommand::PowerOff);
                armed = false;
            }
        }
    }
}

pub fn spawn(spawner: &Spawner) {
    spawner.spawn(auto_shutdown_task().unwrap());
}
