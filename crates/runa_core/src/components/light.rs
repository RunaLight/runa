use crate::Color;
use glam::Vec3;
use runa_macros::Scriptable;

#[derive(Clone, Copy, Debug, Scriptable)]
#[script(crate = "::runa_script_api", builtin)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Color,
    pub intensity: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.3, -1.0, -0.4),
            color: Color::WHITE,
            intensity: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Scriptable)]
#[script(crate = "::runa_script_api", builtin)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub falloff: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 4.0,
            radius: 6.0,
            falloff: 1.0,
        }
    }
}
