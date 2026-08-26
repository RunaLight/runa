use glam::Vec2;
use runa_macros::Scriptable;

use crate::collision2d::rect_corners;
use crate::components::Transform;
use crate::resources::event::Event;
use runa_ecs::Entity;

#[derive(Clone, Copy)]
pub enum Collider2DShape {
    Rect { half_size: Vec2 },
    Circle { radius: f32 },
}

impl Default for Collider2DShape {
    fn default() -> Self {
        Self::Rect {
            half_size: Vec2::ZERO,
        }
    }
}

/// 2D collider attached to an entity. The world-space position/orientation
/// comes from `Transform`; this struct only holds the local shape + flags.
#[derive(Clone, Copy, Scriptable)]
pub struct Collider2D {
    #[script(skip)]
    pub shape: Collider2DShape,
    pub offset: Vec2,
    pub enabled: bool,
    pub is_trigger: bool,
    pub layer: u32,
}

impl Collider2D {
    pub fn new_rect(size: Vec2, offset: Vec2, enabled: bool, is_trigger: bool, layer: u32) -> Self {
        Self {
            shape: Collider2DShape::Rect {
                half_size: size * 0.5,
            },
            offset,
            enabled,
            is_trigger,
            layer,
        }
    }

    pub fn new_circle(
        radius: f32,
        offset: Vec2,
        enabled: bool,
        is_trigger: bool,
        layer: u32,
    ) -> Self {
        Self {
            shape: Collider2DShape::Circle { radius },
            offset,
            enabled,
            is_trigger,
            layer,
        }
    }

    /// Resolve this collider into world space, using the entity's transform.
    /// Called once per entity per frame in the collision system.
    pub fn to_world(&self, t: &Transform) -> WorldCollider2D {
        let center = Vec2::new(t.position.x, t.position.y);
        match self.shape {
            Collider2DShape::Rect { half_size } => WorldCollider2D::Rect {
                corners: rect_corners(center, t.rotation, half_size, self.offset),
            },
            Collider2DShape::Circle { radius } => WorldCollider2D::Circle {
                center: center + self.offset,
                radius,
            },
        }
    }
}

/// A collider already resolved into world space. Typed (no `Option`):
/// `Rect` always carries `corners`, `Circle` always carries `center` + `radius`.
#[derive(Clone, Copy)]
pub enum WorldCollider2D {
    Rect { corners: [Vec2; 4] },
    Circle { center: Vec2, radius: f32 },
}

pub struct OnTriggerEnter2D {
    pub this: Entity,
    pub other: Entity,
}

pub struct OnTriggerExit2D {
    pub this: Entity,
    pub other: Entity,
}

pub struct OnTriggerStay2D {
    pub this: Entity,
    pub other: Entity,
}

impl Event for OnTriggerEnter2D {}
impl Event for OnTriggerExit2D {}
impl Event for OnTriggerStay2D {}
