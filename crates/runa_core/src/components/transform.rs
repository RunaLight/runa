use glam::{Quat, Vec3};

#[derive(Clone, Debug, Default, Copy, runa_macros::Scriptable)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    // for interpolation for fixedframe
    #[script(skip)]
    pub previous_position: Vec3,
    #[script(skip)]
    pub previous_rotation: Quat,
}

impl Transform {
    pub fn rotate_x(&mut self, angle: f32) {
        self.rotation *= Quat::from_rotation_x(angle.to_radians());
    }

    pub fn rotate_y(&mut self, angle: f32) {
        self.rotation *= Quat::from_rotation_y(angle.to_radians());
    }

    pub fn rotate_z(&mut self, angle: f32) {
        self.rotation *= Quat::from_rotation_z(angle.to_radians());
    }

    pub fn prepare_for_update(&mut self) {
        self.previous_position = self.position;
        self.previous_rotation = self.rotation;
    }

    pub fn interpolated_position(&self, interpolation_factor: f32) -> Vec3 {
        Vec3::lerp(
            self.previous_position,
            self.position,
            interpolation_factor.clamp(0.0, 1.0),
        )
    }

    pub fn interpolated_rotation(&self, interpolation_factor: f32) -> Quat {
        self.previous_rotation
            .slerp(self.rotation, interpolation_factor.clamp(0.0, 1.0))
    }

    pub fn sync_previous_to_current(&mut self) {
        self.previous_position = self.position;
        self.previous_rotation = self.rotation;
    }
}
