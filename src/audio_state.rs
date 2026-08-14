//! Audio state machine: connected / idle / playing / paused.

use embassy_executor::Spawner;

use crate::events::{AudioEvent, ShutdownCtrl, AUDIO_EVENTS, SHUTDOWN_CTRL};
use crate::log_line;

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum AudioState {
    Disconnected,
    ConnectedIdle,
    Playing,
    Paused,
}

#[embassy_executor::task]
async fn audio_state_task() {
    let mut state = AudioState::Disconnected;
    log_line::line("audio: Disconnected");

    loop {
        let event = AUDIO_EVENTS.receive().await;
        let next = match (state, event) {
            (_, AudioEvent::Disconnected) => AudioState::Disconnected,
            (AudioState::Disconnected, AudioEvent::Connected) => AudioState::ConnectedIdle,
            (AudioState::ConnectedIdle, AudioEvent::PlaybackStarted)
            | (AudioState::Paused, AudioEvent::PlaybackStarted) => AudioState::Playing,
            (AudioState::Playing, AudioEvent::PlaybackPaused) => AudioState::Paused,
            (AudioState::Playing, AudioEvent::Connected)
            | (AudioState::Paused, AudioEvent::Connected)
            | (AudioState::ConnectedIdle, AudioEvent::Connected) => state,
            (AudioState::Playing, AudioEvent::PlaybackStarted) => AudioState::Playing,
            (AudioState::Paused, AudioEvent::PlaybackPaused) => AudioState::Paused,
            (AudioState::ConnectedIdle, AudioEvent::PlaybackPaused) => AudioState::ConnectedIdle,
            (AudioState::Disconnected, AudioEvent::PlaybackStarted)
            | (AudioState::Disconnected, AudioEvent::PlaybackPaused) => AudioState::Disconnected,
        };

        if next == state {
            continue;
        }

        match next {
            AudioState::Disconnected => log_line::line("BT disconnected"),
            AudioState::ConnectedIdle => log_line::line("BT connected"),
            AudioState::Playing => log_line::line("music playing"),
            AudioState::Paused => log_line::line("music paused"),
        }

        match next {
            AudioState::Playing | AudioState::ConnectedIdle => {
                let _ = SHUTDOWN_CTRL.try_send(ShutdownCtrl::Cancel);
            }
            AudioState::Disconnected | AudioState::Paused => {
                let _ = SHUTDOWN_CTRL.try_send(ShutdownCtrl::Arm);
            }
        }

        state = next;
    }
}

pub fn spawn(spawner: &Spawner) {
    spawner.spawn(audio_state_task().unwrap());
}
