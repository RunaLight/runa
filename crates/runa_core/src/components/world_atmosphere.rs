use crate::Color;
use runa_macros::Scriptable;

#[derive(Clone, Copy, Debug)]
pub enum BackgroundMode {
    SolidColor {
        color: Color,
    },
    VerticalGradient {
        zenith_color: Color,
        horizon_color: Color,
        ground_color: Color,
        horizon_height: f32,
        smoothness: f32,
    },
    /// Reserved for future sky sphere, skybox, or HDR environment map rendering.
    Sky,
}

impl Default for BackgroundMode {
    fn default() -> Self {
        Self::VerticalGradient {
            zenith_color: Color::rgb(0.2, 0.4, 0.8),
            horizon_color: Color::rgb(0.8, 0.9, 1.0),
            ground_color: Color::rgb(0.6, 0.6, 0.7),
            horizon_height: 0.5,
            smoothness: 0.25,
        }
    }
}

#[derive(Clone, Copy, Debug, Scriptable)]
#[script(crate = "::runa_script_api")]
pub struct WorldAtmosphere {
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub background_intensity: f32,
    #[script(skip)]
    pub background: BackgroundMode,
}

impl Default for WorldAtmosphere {
    fn default() -> Self {
        Self {
            ambient_color: Color::WHITE,
            ambient_intensity: 0.15,
            background_intensity: 1.0,
            background: BackgroundMode::default(),
        }
    }
}
