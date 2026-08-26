use glam::{Quat, Vec3};
use runa_macros::Scriptable;

use crate::components::Transform;
use crate::resources::event::Event;
use runa_ecs::Entity;

#[derive(Clone, Copy)]
pub enum Collider3DShape {
    Box { half_size: Vec3 },
    Sphere { radius: f32 },
}

impl Default for Collider3DShape {
    fn default() -> Self {
        Self::Box {
            half_size: Vec3::ZERO,
        }
    }
}

/// 3D collider attached to an entity. World position/orientation comes from
/// `Transform`; this only holds the local shape + flags.
#[derive(Clone, Copy, Scriptable)]
pub struct Collider3D {
    #[script(skip)]
    pub shape: Collider3DShape,
    pub offset: Vec3,
    pub enabled: bool,
    pub is_trigger: bool,
    pub layer: u32,
}

impl Default for Collider3D {
    fn default() -> Self {
        Self::new_box(Vec3::ONE, Vec3::ZERO, true, false, 0)
    }
}

impl Collider3D {
    pub fn new_box(size: Vec3, offset: Vec3, enabled: bool, is_trigger: bool, layer: u32) -> Self {
        Self {
            shape: Collider3DShape::Box {
                half_size: size * 0.5,
            },
            offset,
            enabled,
            is_trigger,
            layer,
        }
    }

    pub fn new_sphere(
        radius: f32,
        offset: Vec3,
        enabled: bool,
        is_trigger: bool,
        layer: u32,
    ) -> Self {
        Self {
            shape: Collider3DShape::Sphere { radius },
            offset,
            enabled,
            is_trigger,
            layer,
        }
    }

    /// Resolve this collider into world space, using the entity's transform.
    pub fn to_world(&self, t: &Transform) -> WorldCollider3D {
        let center = t.position + t.rotation * self.offset;
        match self.shape {
            Collider3DShape::Box { half_size } => WorldCollider3D::Box {
                center,
                half_size,
                rotation: t.rotation,
            },
            Collider3DShape::Sphere { radius } => WorldCollider3D::Sphere { center, radius },
        }
    }
}

/// A collider already resolved into world space. For `Box` we keep the
/// orientation (Quat) + half extents instead of 8 corners — SAT reads the
/// 3 world axes straight from `rotation`.
#[derive(Clone, Copy)]
pub enum WorldCollider3D {
    Box {
        center: Vec3,
        half_size: Vec3,
        rotation: Quat,
    },
    Sphere {
        center: Vec3,
        radius: f32,
    },
}

/// System-owned resource remembering each entity's 3D contacts from the
/// previous frame, so enter/exit/stay can be diffed. (Defined in
/// `resources::collision_tracker`.)
pub struct OnTriggerEnter3D {
    pub this: Entity,
    pub other: Entity,
}

pub struct OnTriggerExit3D {
    pub this: Entity,
    pub other: Entity,
}

pub struct OnTriggerStay3D {
    pub this: Entity,
    pub other: Entity,
}

impl Event for OnTriggerEnter3D {}
impl Event for OnTriggerExit3D {}
impl Event for OnTriggerStay3D {}
