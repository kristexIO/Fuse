use crate::error::{FuseError, FuseResult};
use crate::models::{PlaybackQueueItem, PlaybackState};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::fs::File;
use std::path::Path;
use std::time::Duration;

const ENGINE_NAME: &str = "rodio";

pub struct PlaybackEngine {
    stream: Option<MixerDeviceSink>,
    player: Option<Player>,
    queue: Vec<PlaybackQueueItem>,
    queue_index: Option<usize>,
    volume: f32,
    status: PlaybackStatus,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackStatus {
    Idle,
    Playing,
    Paused,
    Stopped,
    Error,
}

impl PlaybackStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
            Self::Error => "error",
        }
    }
}

impl Default for PlaybackEngine {
    fn default() -> Self {
        Self {
            stream: None,
            player: None,
            queue: Vec::new(),
            queue_index: None,
            volume: 0.72,
            status: PlaybackStatus::Idle,
            error: None,
        }
    }
}

impl PlaybackEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_queue(
        &mut self,
        queue: Vec<PlaybackQueueItem>,
        start_index: Option<usize>,
    ) -> FuseResult<PlaybackState> {
        self.queue = queue;
        self.queue_index = if self.queue.is_empty() {
            None
        } else {
            Some(start_index.unwrap_or_default().min(self.queue.len() - 1))
        };
        self.error = None;

        if self.queue.is_empty() {
            self.stop_sink();
            self.status = PlaybackStatus::Idle;
        }

        Ok(self.state())
    }

    pub fn play_queue_index(&mut self, index: usize) -> FuseResult<PlaybackState> {
        if index >= self.queue.len() {
            return Err(FuseError::Validation(
                "Playback queue index is out of range".to_string(),
            ));
        }

        self.queue_index = Some(index);
        self.play_current()
    }

    pub fn play_track(&mut self, item: PlaybackQueueItem) -> FuseResult<PlaybackState> {
        self.queue = vec![item];
        self.queue_index = Some(0);
        self.play_current()
    }

    pub fn pause(&mut self) -> PlaybackState {
        if let Some(player) = &self.player {
            player.pause();
            self.status = PlaybackStatus::Paused;
        }

        self.state()
    }

    pub fn resume(&mut self) -> FuseResult<PlaybackState> {
        if self.player.is_none() && self.queue_index.is_some() {
            return self.play_current();
        }

        if let Some(player) = &self.player {
            player.play();
            self.status = PlaybackStatus::Playing;
        }

        Ok(self.state())
    }

    pub fn stop(&mut self) -> PlaybackState {
        self.stop_sink();
        self.status = PlaybackStatus::Stopped;
        self.state()
    }

    pub fn seek(&mut self, position_ms: i64) -> FuseResult<PlaybackState> {
        let Some(player) = &self.player else {
            return Ok(self.state());
        };

        let position_ms = position_ms.max(0) as u64;
        player
            .try_seek(Duration::from_millis(position_ms))
            .map_err(|error| FuseError::Playback(error.to_string()))?;

        Ok(self.state())
    }

    pub fn set_volume(&mut self, volume: f32) -> PlaybackState {
        self.volume = volume.clamp(0.0, 1.0);

        if let Some(player) = &self.player {
            player.set_volume(self.volume);
        }

        self.state()
    }

    pub fn state(&self) -> PlaybackState {
        let current = self.current_item();
        let position_ms = self
            .player
            .as_ref()
            .map(|player| player.get_pos().as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or_default();

        PlaybackState {
            engine: ENGINE_NAME.to_string(),
            status: self.effective_status().as_str().to_string(),
            track_id: current.map(|item| item.track_id),
            position_ms,
            duration_ms: current.and_then(|item| item.duration_ms),
            volume: self.volume,
            queue: self.queue.iter().map(|item| item.track_id).collect(),
            queue_index: self.queue_index,
            error: self.error.clone(),
        }
    }

    fn play_current(&mut self) -> FuseResult<PlaybackState> {
        let item = self
            .current_item()
            .cloned()
            .ok_or_else(|| FuseError::Validation("Playback queue is empty".to_string()))?;

        if !Path::new(&item.path).exists() {
            let message = "Track file is missing on disk".to_string();
            self.error = Some(message.clone());
            self.status = PlaybackStatus::Error;
            return Err(FuseError::Playback(message));
        }

        self.ensure_stream()?;
        let file = File::open(&item.path)?;
        let source =
            Decoder::try_from(file).map_err(|error| FuseError::Playback(error.to_string()))?;

        self.stop_sink();
        let stream = self.ensure_stream()?;
        let player = Player::connect_new(stream.mixer());
        player.set_volume(self.volume);
        player.append(source);
        player.play();

        self.player = Some(player);
        self.status = PlaybackStatus::Playing;
        self.error = None;
        Ok(self.state())
    }

    fn ensure_stream(&mut self) -> FuseResult<&mut MixerDeviceSink> {
        if self.stream.is_none() {
            self.stream = Some(
                DeviceSinkBuilder::open_default_sink()
                    .map_err(|error| FuseError::Playback(error.to_string()))?,
            );
        }

        Ok(self.stream.as_mut().expect("stream is initialized"))
    }

    fn current_item(&self) -> Option<&PlaybackQueueItem> {
        self.queue_index.and_then(|index| self.queue.get(index))
    }

    fn stop_sink(&mut self) {
        if let Some(player) = self.player.take() {
            player.stop();
        }
    }

    fn effective_status(&self) -> PlaybackStatus {
        if matches!(self.status, PlaybackStatus::Playing) {
            if let Some(player) = &self.player {
                if player.is_paused() {
                    return PlaybackStatus::Paused;
                }

                if player.empty() {
                    return PlaybackStatus::Stopped;
                }
            }
        }

        self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(track_id: i64) -> PlaybackQueueItem {
        PlaybackQueueItem {
            track_id,
            path: format!("C:/missing/{track_id}.mp3"),
            title: format!("Track {track_id}"),
            artist: None,
            duration_ms: Some(120_000),
        }
    }

    #[test]
    fn queue_state_tracks_selected_index() {
        let mut engine = PlaybackEngine::new();
        let state = engine.set_queue(vec![item(1), item(2)], Some(1)).unwrap();

        assert_eq!(state.queue, vec![1, 2]);
        assert_eq!(state.queue_index, Some(1));
        assert_eq!(state.track_id, Some(2));
    }

    #[test]
    fn volume_is_clamped() {
        let mut engine = PlaybackEngine::new();

        assert_eq!(engine.set_volume(2.0).volume, 1.0);
        assert_eq!(engine.set_volume(-1.0).volume, 0.0);
    }
}
