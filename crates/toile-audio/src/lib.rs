use std::collections::HashMap;
use std::path::{Path, PathBuf};

use kira::manager::backend::DefaultBackend;
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::tween::Tween;
use kira::Volume;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Audio backend error: {0}")]
    Backend(String),
    #[error("Failed to load audio: {0}")]
    Load(String),
    #[error("Failed to play: {0}")]
    Play(String),
    #[error("Invalid sound ID")]
    InvalidSound,
    #[error("Invalid music ID")]
    InvalidMusic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MusicId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaybackId(u32);

enum ActivePlayback {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<kira::sound::FromFileError>),
}

/// Audio subsystem wrapping kira. Provides load/play/stop/pause/volume.
pub struct Audio {
    manager: AudioManager,
    sounds: Vec<StaticSoundData>,
    music_paths: Vec<PathBuf>,
    playbacks: HashMap<PlaybackId, ActivePlayback>,
    next_playback_id: u32,
    master_vol: f32,
}

impl Audio {
    pub fn new() -> Result<Self, AudioError> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| AudioError::Backend(e.to_string()))?;

        log::info!("Audio subsystem initialized");

        Ok(Self {
            manager,
            sounds: Vec::new(),
            music_paths: Vec::new(),
            playbacks: HashMap::new(),
            next_playback_id: 0,
            master_vol: 1.0,
        })
    }

    /// Load a sound effect fully into memory (WAV, OGG, etc.).
    pub fn load_sound(&mut self, path: &Path) -> Result<SoundId, AudioError> {
        let data = StaticSoundData::from_file(path)
            .map_err(|e| AudioError::Load(format!("{}: {e}", path.display())))?;
        let id = SoundId(self.sounds.len() as u32);
        self.sounds.push(data);
        log::info!("Loaded sound: {} -> {:?}", path.display(), id);
        Ok(id)
    }

    /// Register a music track for streaming playback (path validated on load).
    pub fn load_music(&mut self, path: &Path) -> Result<MusicId, AudioError> {
        // Validate the file is readable
        let _ = StreamingSoundData::from_file(path)
            .map_err(|e| AudioError::Load(format!("{}: {e}", path.display())))?;
        let id = MusicId(self.music_paths.len() as u32);
        self.music_paths.push(path.to_path_buf());
        log::info!("Loaded music: {} -> {:?}", path.display(), id);
        Ok(id)
    }

    /// Play a sound effect once.
    pub fn play_sound(&mut self, id: SoundId) -> Result<PlaybackId, AudioError> {
        let data = self
            .sounds
            .get(id.0 as usize)
            .ok_or(AudioError::InvalidSound)?
            .clone();
        let handle = self
            .manager
            .play(data)
            .map_err(|e| AudioError::Play(e.to_string()))?;
        self.insert_playback(ActivePlayback::Static(handle))
    }

    /// Play a sound effect in a loop.
    pub fn play_sound_looped(&mut self, id: SoundId) -> Result<PlaybackId, AudioError> {
        let data = self
            .sounds
            .get(id.0 as usize)
            .ok_or(AudioError::InvalidSound)?
            .clone()
            .loop_region(..);
        let handle = self
            .manager
            .play(data)
            .map_err(|e| AudioError::Play(e.to_string()))?;
        self.insert_playback(ActivePlayback::Static(handle))
    }

    /// Play a music track (streaming, looped).
    pub fn play_music(&mut self, id: MusicId) -> Result<PlaybackId, AudioError> {
        let path = self
            .music_paths
            .get(id.0 as usize)
            .ok_or(AudioError::InvalidMusic)?
            .clone();
        let data = StreamingSoundData::from_file(&path)
            .map_err(|e| AudioError::Load(e.to_string()))?
            .loop_region(..);
        let handle = self
            .manager
            .play(data)
            .map_err(|e| AudioError::Play(e.to_string()))?;
        self.insert_playback(ActivePlayback::Streaming(handle))
    }

    /// Play a music track once (no loop).
    pub fn play_music_once(&mut self, id: MusicId) -> Result<PlaybackId, AudioError> {
        let path = self
            .music_paths
            .get(id.0 as usize)
            .ok_or(AudioError::InvalidMusic)?
            .clone();
        let data = StreamingSoundData::from_file(&path)
            .map_err(|e| AudioError::Load(e.to_string()))?;
        let handle = self
            .manager
            .play(data)
            .map_err(|e| AudioError::Play(e.to_string()))?;
        self.insert_playback(ActivePlayback::Streaming(handle))
    }

    pub fn stop(&mut self, id: PlaybackId) {
        if let Some(playback) = self.playbacks.remove(&id) {
            let tw = Tween::default();
            match playback {
                ActivePlayback::Static(mut h) => {
                    h.stop(tw);
                }
                ActivePlayback::Streaming(mut h) => {
                    h.stop(tw);
                }
            }
        }
    }

    pub fn pause(&mut self, id: PlaybackId) {
        if let Some(playback) = self.playbacks.get_mut(&id) {
            let tw = Tween::default();
            match playback {
                ActivePlayback::Static(h) => {
                    h.pause(tw);
                }
                ActivePlayback::Streaming(h) => {
                    h.pause(tw);
                }
            }
        }
    }

    pub fn resume(&mut self, id: PlaybackId) {
        if let Some(playback) = self.playbacks.get_mut(&id) {
            let tw = Tween::default();
            match playback {
                ActivePlayback::Static(h) => {
                    h.resume(tw);
                }
                ActivePlayback::Streaming(h) => {
                    h.resume(tw);
                }
            }
        }
    }

    /// Set volume for a playing sound (0.0 = silent, 1.0 = full).
    pub fn set_volume(&mut self, id: PlaybackId, volume: f32) {
        let vol = linear_to_volume(volume);
        let tw = Tween::default();
        if let Some(playback) = self.playbacks.get_mut(&id) {
            match playback {
                ActivePlayback::Static(h) => {
                    h.set_volume(vol, tw);
                }
                ActivePlayback::Streaming(h) => {
                    h.set_volume(vol, tw);
                }
            }
        }
    }

    /// Set master volume (0.0 = silent, 1.0 = full).
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_vol = volume.clamp(0.0, 1.0);
        let vol = linear_to_volume(self.master_vol);
        self.manager.main_track().set_volume(vol, Tween::default());
    }

    pub fn master_volume(&self) -> f32 {
        self.master_vol
    }

    fn insert_playback(&mut self, playback: ActivePlayback) -> Result<PlaybackId, AudioError> {
        let id = PlaybackId(self.next_playback_id);
        self.next_playback_id += 1;
        self.playbacks.insert(id, playback);
        Ok(id)
    }
}

fn linear_to_volume(amplitude: f32) -> Volume {
    if amplitude <= 0.0001 {
        Volume::Decibels(Volume::MIN_DECIBELS)
    } else {
        Volume::Decibels(20.0 * amplitude.clamp(0.0001, 1.0).log10() as f64)
    }
}
