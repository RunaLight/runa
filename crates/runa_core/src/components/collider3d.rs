use glam::{Quat, Vec3};
use runa_macros::Scriptable;
use runa_script_api::luau::{FromLua, IntoLua, LuaRef, Result, Table, Value};

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

impl<'lua> IntoLua<'lua> for Collider3DShape {
    fn into_lua(self, lua: LuaRef<'lua>) -> Result<Value<'lua>> {
        let t = lua.create_table()?;
        match self {
            Collider3DShape::Box { half_size } => {
                t.set("type", "Box")?;
                let hs = lua.create_table()?;
                hs.set("x", half_size.x)?;
                hs.set("y", half_size.y)?;
                hs.set("z", half_size.z)?;
                t.set("half_size", hs)?;
            }
            Collider3DShape::Sphere { radius } => {
                t.set("type", "Sphere")?;
                t.set("radius", radius)?;
            }
        }
        Ok(Value::Table(t))
    }
}

impl<'lua> FromLua<'lua> for Collider3DShape {
    fn from_lua(value: Value<'lua>, lua: LuaRef<'lua>) -> Result<Self> {
        if let Value::Nil = value {
            return Ok(Self::default());
        }
        let t = Table::from_lua(value, lua)?;
        let kind: String = t.get("type").unwrap_or_default();
        match kind.as_str() {
            "Box" => {
                let hs =
                    t.get::<Table>("half_size").unwrap_or_else(|_| lua.create_table().unwrap());
                let x: f32 = hs.get("x").unwrap_or(0.0);
                let y: f32 = hs.get("y").unwrap_or(0.0);
                let z: f32 = hs.get("z").unwrap_or(0.0);
                Ok(Collider3DShape::Box {
                    half_size: Vec3::new(x, y, z),
                })
            }
            "Sphere" => {
                let radius: f32 = t.get("radius").unwrap_or(0.0);
                Ok(Collider3DShape::Sphere { radius })
            }
            _ => Ok(Collider3DShape::default()),
        }
    }
}

/// 3D collider attached to an entity. World position/orientation comes from
/// `Transform`; this only holds the local shape + flags.
#[derive(Clone, Copy, Scriptable)]
#[script(crate = "::runa_script_api")]
pub struct Collider3D {
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
