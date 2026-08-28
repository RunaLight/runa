/// Audio listener component — represents the "ears" of the audio system
///
/// Attach this to your camera or player object to enable 3D spatial audio.
/// Only one AudioListener should exist in the world at a time.
///
/// The listener's position and rotation affect how 3D sounds are heard:
/// - Sounds to the left will be quieter (simulated stereo)
/// - Sounds to the right will be quieter (simulated stereo)
/// - Distance affects volume attenuation
///
/// # Note on Stereo Panning
///
/// True stereo panning (separate left/right channels) requires audio backend
/// support. Currently, the engine uses volume attenuation based on position
/// to simulate directionality. For true stereo, consider upgrading the audio
/// backend.
use runa_macros::Scriptable;

#[derive(Clone, Scriptable)]
#[script(crate = "::runa_script_api")]
pub struct AudioListener {
    /// Listener volume (0.0 to 1.0)
    pub volume: f32,
    /// Is this listener active (only one active listener at a time)
    pub active: bool,
    /// Stereo separation factor (0.0 = mono, 1.0 = full stereo panning)
    ///
    /// Controls how much sounds pan between left and right channels based on
    /// their position relative to the listener. Higher values mean more
    /// extreme panning.
    pub stereo_separation: f32,
}

impl AudioListener {
    /// Create a new audio listener
    pub fn new() -> Self {
        Self {
            volume: 1.0,
            active: true,
            stereo_separation: 1.0, // Full stereo by default
        }
    }

    /// Create a new audio listener with custom volume
    pub fn with_volume(volume: f32) -> Self {
        Self {
            volume: volume.clamp(0.0, 1.0),
            active: true,
            stereo_separation: 1.0,
        }
    }

    /// Create a new audio listener with custom stereo separation
    pub fn with_stereo_separation(separation: f32) -> Self {
        Self {
            volume: 1.0,
            active: true,
            stereo_separation: separation.clamp(0.0, 1.0),
        }
    }
}

impl Default for AudioListener {
    fn default() -> Self {
        Self::new()
    }
}
