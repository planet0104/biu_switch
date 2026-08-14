//! 音箱事件通道：音频状态与电源命令。

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum AudioEvent {
    Connected,
    Disconnected,
    PlaybackStarted,
    PlaybackPaused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum PowerCommand {
    PowerOff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum ShutdownCtrl {
    /// 开始/重启 10 分钟倒计时
    Arm,
    /// 取消倒计时（已连接或正在播放）
    Cancel,
}

pub static AUDIO_EVENTS: Channel<CriticalSectionRawMutex, AudioEvent, 8> = Channel::new();
pub static POWER_CMDS: Channel<CriticalSectionRawMutex, PowerCommand, 4> = Channel::new();
pub static SHUTDOWN_CTRL: Channel<CriticalSectionRawMutex, ShutdownCtrl, 4> = Channel::new();
