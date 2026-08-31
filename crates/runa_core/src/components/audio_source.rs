use crate::audio::SoundId;
use runa_asset::AudioAsset;
use runa_macros::Scriptable;
use std::sync::Arc;

/// Audio source component — attaches audio to a game object
#[derive(Clone, Default, Scriptable)]
#[script(crate = "::runa_script_api", not_addable, builtin)]
pub struct AudioSource {
    /// Cached audio asset (decoded PCM samples)
    #[script(skip)]
    pub audio_asset: Option<Arc<AudioAsset>>,
    /// Relative asset path used to reload this source in editor/runtime
    pub source_path: Option<String>,
    /// Playback volume (0.0 to 1.0)
    pub volume: f32,
    /// Loop playback
    pub looped: bool,
    /// Is currently playing
    pub playing: bool,
    /// Play automatically when object spawns
    pub play_on_awake: bool,
    /// Request playback on next world update
    pub play_requested: bool,
    /// Request stop on next world update
    pub stop_requested: bool,
    /// Current sound ID if playing
    #[script(skip)]
    pub sound_id: Option<SoundId>,
    /// Minimum distance for sound attenuation (3D sound)
    pub min_distance: f32,
    /// Maximum distance for sound attenuation (3D sound)
    pub max_distance: f32,
    /// Is this a 3D sound (affected by position and listener)
    pub spatial: bool,
}

impl AudioSource {
    /// Create empty audio source (2D sound)
    pub fn new2d() -> Self {
        Self {
            audio_asset: None,
            source_path: None,
            volume: 1.0,
            looped: false,
            playing: false,
            play_on_awake: false,
            play_requested: false,
            stop_requested: false,
            sound_id: None,
            min_distance: 1.0,
            max_distance: 100.0,
            spatial: false,
        }
    }

    /// Create empty audio source (3D spatial sound)
    pub fn new3d() -> Self {
        Self {
            audio_asset: None,
            source_path: None,
            volume: 1.0,
            looped: false,
            playing: false,
            play_on_awake: false,
            play_requested: false,
            stop_requested: false,
            sound_id: None,
            min_distance: 1.0,
            max_distance: 100.0,
            spatial: true,
        }
    }

    /// Set audio asset
    pub fn with_asset(audio_asset: Arc<AudioAsset>) -> Self {
        Self {
            audio_asset: Some(audio_asset),
            source_path: None,
            volume: 1.0,
            looped: false,
            playing: false,
            play_on_awake: false,
            play_requested: false,
            stop_requested: false,
            sound_id: None,
            min_distance: 1.0,
            max_distance: 100.0,
            spatial: false,
        }
    }

    /// Set audio asset for 3D sound
    pub fn with_asset_3d(audio_asset: Arc<AudioAsset>) -> Self {
        Self {
            audio_asset: Some(audio_asset),
            source_path: None,
            volume: 1.0,
            looped: false,
            playing: false,
            play_on_awake: false,
            play_requested: false,
            stop_requested: false,
            sound_id: None,
            min_distance: 1.0,
            max_distance: 100.0,
            spatial: true,
        }
    }

    /// Set audio asset
    pub fn set_asset(&mut self, audio_asset: Arc<AudioAsset>) {
        self.audio_asset = Some(audio_asset);
    }

    pub fn set_asset_with_path(
        &mut self,
        audio_asset: Option<Arc<AudioAsset>>,
        source_path: Option<String>,
    ) {
        self.audio_asset = audio_asset;
        self.source_path = source_path;
    }

    /// Request playback. The sound will be played on the next world update.
    pub fn play(&mut self) {
        self.play_requested = true;
        self.stop_requested = false;
    }

    /// Request stop. The sound will be stopped on the next world update.
    pub fn stop(&mut self) {
        self.stop_requested = true;
        self.play_requested = false;
    }
}
