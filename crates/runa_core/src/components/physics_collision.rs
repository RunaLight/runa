use glam::Vec2;
use runa_macros::Scriptable;

#[derive(Clone, Default, Scriptable)]
pub struct PhysicsCollision {
    pub size: Vec2, // половина размера (extents)
    pub enabled: bool,
}

impl PhysicsCollision {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            size: Vec2::new(width * 0.5, height * 0.5),
            enabled: true,
        }
    }

    pub fn contains_point(&self, point: Vec2, center: Vec2) -> bool {
        if !self.enabled {
            return false;
        }

        let min_x = center.x - self.size.x;
        let max_x = center.x + self.size.x;
        let min_y = center.y - self.size.y;
        let max_y = center.y + self.size.y;

        point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
    }
}
