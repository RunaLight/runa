use runa_macros::Scriptable;
use runa_render_api::{ScreenEffectData, ScreenEffectFlags};

#[derive(Clone, Debug, Scriptable)]
pub struct ScreenEffects {
    #[script(skip)]
    pub enabled: ScreenEffectFlags,
    pub fade_color: [f32; 4],
    pub vignette_strength: f32,
    pub vignette_radius: f32,
    pub vignette_softness: f32,
    pub rgb_shift: [f32; 2],
    pub tint_color: [f32; 4],
    pub brightness: f32,
    pub contrast: f32,
}

impl Default for ScreenEffects {
    fn default() -> Self {
        Self {
            enabled: ScreenEffectFlags::default(),
            fade_color: [0.0, 0.0, 0.0, 0.0],
            vignette_strength: 0.0,
            vignette_radius: 0.5,
            vignette_softness: 0.2,
            rgb_shift: [0.0, 0.0],
            tint_color: [1.0, 1.0, 1.0, 1.0],
            brightness: 1.0,
            contrast: 1.0,
        }
    }
}

impl ScreenEffects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fade(mut self, color: [f32; 4]) -> Self {
        self.fade_color = color;
        self.enabled.fade = color[3] > 0.0;
        self
    }

    pub fn set_fade(&mut self, color: [f32; 4]) {
        self.fade_color = color;
        self.enabled.fade = color[3] > 0.0;
    }

    pub fn with_vignette(mut self, strength: f32, radius: f32, softness: f32) -> Self {
        self.vignette_strength = strength;
        self.vignette_radius = radius;
        self.vignette_softness = softness;
        self.enabled.vignette = strength > 0.0;
        self
    }

    pub fn set_vignette(&mut self, strength: f32, radius: f32, softness: f32) {
        self.vignette_strength = strength;
        self.vignette_radius = radius;
        self.vignette_softness = softness;
        self.enabled.vignette = strength > 0.0;
    }

    pub fn with_rgb_shift(mut self, shift_x: f32, shift_y: f32) -> Self {
        self.rgb_shift = [shift_x, shift_y];
        self.enabled.rgb_shift = shift_x != 0.0 || shift_y != 0.0;
        self
    }

    pub fn set_rgb_shift(&mut self, shift_x: f32, shift_y: f32) {
        self.rgb_shift = [shift_x, shift_y];
        self.enabled.rgb_shift = shift_x != 0.0 || shift_y != 0.0;
    }

    pub fn with_tint(mut self, color: [f32; 4]) -> Self {
        self.tint_color = color;
        self.enabled.tint = true;
        self
    }

    pub fn set_tint(&mut self, color: [f32; 4]) {
        self.tint_color = color;
        self.enabled.tint = true;
    }

    pub fn clear_tint(&mut self) {
        self.tint_color = [1.0, 1.0, 1.0, 1.0];
        self.enabled.tint = false;
    }

    pub fn disable_all(&mut self) {
        self.enabled = ScreenEffectFlags::default();
    }

    pub fn to_render_data(&self) -> ScreenEffectData {
        ScreenEffectData {
            fade_color: self.fade_color,
            vignette_strength: self.vignette_strength,
            vignette_radius: self.vignette_radius,
            vignette_softness: self.vignette_softness,
            rgb_shift: self.rgb_shift,
            tint_color: self.tint_color,
            brightness: self.brightness,
            contrast: self.contrast,
            enabled: self.enabled,
        }
    }
}
