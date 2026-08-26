//! Low-level bridge between Rust and Luau.
//!
//! Defines [`ScriptType`] (a compile-time registration record for a scriptable
//! Rust type) and the `inventory`-based collection that lets any crate mark its
//! types as scriptable without creating a dependency back to `runa_engine::scripting`.
//!
//! Also provides math conversions (`glam` <-> Luau tables) and re-exports the
//! `luau` embedding API so derive-generated code can stay decoupled from the
//! concrete `luau` crate version.

use runa_ecs::{Entity, World};

// Re-export the embedding API so generated code only needs `runa_script_api`.
pub use inventory;
pub use inventory::{collect, iter, submit};
pub use luau;

/// A scriptable Rust type: its Luau type definition string plus the functions
/// that project it into / out of a Luau table for a given `Entity`.
pub struct ScriptType {
    pub name: &'static str,
    pub type_def: &'static str,
    pub to_luau: for<'lua> fn(&'lua luau::Lua, &World, Entity) -> Option<luau::Table<'lua>>,
    pub from_luau: for<'lua> fn(&'lua luau::Lua, luau::Value<'lua>, &mut World, Entity),
}

impl Clone for ScriptType {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for ScriptType {}

inventory::collect!(ScriptType);

/// Math type conversions between `glam` and Luau tables. These are free
/// functions (not trait impls) so we avoid the orphan rule — the derive macro
/// emits calls to them for `Vec2`/`Vec3`/`Vec4`/`Quat` fields.
pub mod math {
    use glam::{Quat, Vec2, Vec3, Vec4};
    use luau::{Error, LuaRef, Result, Table};

    pub fn vec2_to_luau(lua: LuaRef<'_>, v: Vec2) -> Result<Table<'_>> {
        let t = lua.create_table()?;
        t.set("x", v.x as f64)?;
        t.set("y", v.y as f64)?;
        Ok(t)
    }

    pub fn vec2_from_luau(t: &Table) -> Vec2 {
        Vec2::new(
            t.get::<f64>("x").unwrap_or(0.0) as f32,
            t.get::<f64>("y").unwrap_or(0.0) as f32,
        )
    }

    pub fn vec3_to_luau(lua: LuaRef<'_>, v: Vec3) -> Result<Table<'_>> {
        let t = lua.create_table()?;
        t.set("x", v.x as f64)?;
        t.set("y", v.y as f64)?;
        t.set("z", v.z as f64)?;
        Ok(t)
    }

    pub fn vec3_from_luau(t: &Table) -> Vec3 {
        Vec3::new(
            t.get::<f64>("x").unwrap_or(0.0) as f32,
            t.get::<f64>("y").unwrap_or(0.0) as f32,
            t.get::<f64>("z").unwrap_or(0.0) as f32,
        )
    }

    pub fn vec4_to_luau(lua: LuaRef<'_>, v: Vec4) -> Result<Table<'_>> {
        let t = lua.create_table()?;
        t.set("x", v.x as f64)?;
        t.set("y", v.y as f64)?;
        t.set("z", v.z as f64)?;
        t.set("w", v.w as f64)?;
        Ok(t)
    }

    pub fn vec4_from_luau(t: &Table) -> Vec4 {
        Vec4::new(
            t.get::<f64>("x").unwrap_or(0.0) as f32,
            t.get::<f64>("y").unwrap_or(0.0) as f32,
            t.get::<f64>("z").unwrap_or(0.0) as f32,
            t.get::<f64>("w").unwrap_or(0.0) as f32,
        )
    }

    pub fn quat_to_luau(lua: LuaRef<'_>, q: Quat) -> Result<Table<'_>> {
        vec4_to_luau(lua, Vec4::new(q.x, q.y, q.z, q.w))
    }

    pub fn quat_from_luau(t: &Table) -> Quat {
        let v = vec4_from_luau(t);
        Quat::from_xyzw(v.x, v.y, v.z, v.w)
    }

    /// Used by error-free fallbacks when a math field is missing/garbage.
    pub fn err_type(name: &str) -> Error {
        Error::runtime(format!("scriptable: expected table for {name}"))
    }
}
