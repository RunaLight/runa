use glam::Vec2;
use runa_macros::Scriptable;
use runa_script_api::luau::{FromLua, IntoLua, LuaRef, Result, Table, Value};

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

impl<'lua> IntoLua<'lua> for Collider2DShape {
    fn into_lua(self, lua: LuaRef<'lua>) -> Result<Value<'lua>> {
        let t = lua.create_table()?;
        match self {
            Collider2DShape::Rect { half_size } => {
                t.set("type", "Rect")?;
                let hs = lua.create_table()?;
                hs.set("x", half_size.x)?;
                hs.set("y", half_size.y)?;
                t.set("half_size", hs)?;
            }
            Collider2DShape::Circle { radius } => {
                t.set("type", "Circle")?;
                t.set("radius", radius)?;
            }
        }
        Ok(Value::Table(t))
    }
}

impl<'lua> FromLua<'lua> for Collider2DShape {
    fn from_lua(value: Value<'lua>, lua: LuaRef<'lua>) -> Result<Self> {
        if let Value::Nil = value {
            return Ok(Self::default());
        }
        let t = Table::from_lua(value, lua)?;
        let kind: String = t.get("type").unwrap_or_default();
        match kind.as_str() {
            "Rect" => {
                let hs = t.get::<Table>("half_size").unwrap_or_else(|_| lua.create_table().unwrap());
                let x: f32 = hs.get("x").unwrap_or(0.0);
                let y: f32 = hs.get("y").unwrap_or(0.0);
                Ok(Collider2DShape::Rect {
                    half_size: Vec2::new(x, y),
                })
            }
            "Circle" => {
                let radius: f32 = t.get("radius").unwrap_or(0.0);
                Ok(Collider2DShape::Circle { radius })
            }
            _ => Ok(Collider2DShape::default()),
        }
    }
}

/// 2D collider attached to an entity. The world-space position/orientation
/// comes from `Transform`; this struct only holds the local shape + flags.
#[derive(Clone, Copy, Default, Scriptable)]
#[script(crate = "::runa_script_api", not_addable, builtin)]
pub struct Collider2D {
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

// Luau type definition for the tagged-union collider shape (2D).
runa_script_api::submit!(runa_script_api::ScriptAuxType {
    name: "Collider2DShape",
    type_def: "--- 2D collider shape: a rect (half-size) or a circle.\nexport type Collider2DShape = { type: \"Rect\", half_size: Vec2 } | { type: \"Circle\", radius: number }\n",
    builtin: true,
});
