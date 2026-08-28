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
    pub to_luau: for<'lua> fn(luau::LuaRef<'lua>, &World, Entity) -> Option<luau::Table<'lua>>,
    pub from_luau: for<'lua> fn(luau::LuaRef<'lua>, luau::Value<'lua>, &mut World, Entity),
    /// Insert-or-create this component from a Luau table (`ctx:AddComponent`).
    pub add: for<'lua> fn(luau::LuaRef<'lua>, luau::Value<'lua>, &mut World, Entity),
    /// Remove this component from an entity (`ctx:RemoveComponent`).
    pub remove: fn(&mut World, Entity),
}

impl Clone for ScriptType {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for ScriptType {}

inventory::collect!(ScriptType);

/// A Rust function exposed to Luau via `#[script_fn]`. The engine registers every
/// collected instance onto the `runa` module (and as a bare global) when it builds a
/// VM, so scripts can call `runa.my_function(...)` / `my_function(...)`.
pub struct ScriptFunction {
    pub name: &'static str,
    /// Uniform callback shape: all Luau arguments arrive as a `Variadic<Value>` and
    /// the result is returned as a single `Value`. The `#[script_fn]` macro generates
    /// the glue that converts arguments/return value to/from Rust types.
    pub func: for<'lua> fn(
        lua: luau::LuaRef<'lua>,
        args: luau::Variadic<luau::Value<'lua>>,
    ) -> luau::Result<luau::Value<'lua>>,
}

impl Clone for ScriptFunction {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for ScriptFunction {}

inventory::collect!(ScriptFunction);

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

/// Regenerates the Luau type-definition module (`runa.luau`) that `luau-lsp`
/// reads for autocomplete / type checking. Call this from your app's startup
/// (once) with the directory your `.luau` scripts live in, e.g.
/// `write_luau_types(Path::new("scripts/runa.luau"))`.
///
/// Everything here is derived automatically:
/// - math types (`Vec3`, `Vec4`, `Vec2`, `Quat`),
/// - the `KeyCode` union + `ScriptInput`,
/// - every `#[derive(Scriptable)]` type (collected via `inventory`),
/// - `ScriptContext` and the component "class" globals (`Transform`, ...).
pub fn write_luau_types(path: &std::path::Path) {
    let mut s = String::new();
    s.push_str("-- THIS FILE IS AUTO-GENERATED. DO NOT EDIT BY HAND.\n");
    s.push_str(
        "-- Source of truth: `#[derive(Scriptable)]` registrations (collected via `inventory`)\n",
    );
    s.push_str("-- plus built-in math / KeyCode types. Read by `luau-lsp` for type checking.\n\n");

    // Built-in math value types.
    s.push_str("export type Vec3 = { x: number, y: number, z: number }\n");
    s.push_str("export type Vec4 = { x: number, y: number, z: number, w: number }\n");
    s.push_str("export type Vec2 = { x: number, y: number }\n");
    s.push_str("export type Quat = { x: number, y: number, z: number, w: number }\n\n");

    // KeyCode union + ScriptInput (every key is an optional boolean field).
    let kc = keycode_names();
    let union = kc
        .iter()
        .map(|k| format!("\"{k}\""))
        .collect::<Vec<_>>()
        .join(" | ");
    s.push_str(&format!("export type KeyCode = {union}\n\n"));
    s.push_str("export type ScriptInput = {\n");
    for k in kc {
        s.push_str(&format!("    {k}: boolean,\n"));
    }
    // Mirrored engine state (keyed by `KeyCode` / mouse-button name strings).
    s.push_str("    keys_just_pressed: { [string]: boolean },\n");
    s.push_str("    mouse_buttons_pressed: { [string]: boolean },\n");
    s.push_str("    mouse_buttons_just_pressed: { [string]: boolean },\n");
    s.push_str("    mouse_position: { x: number, y: number },\n");
    s.push_str("    mouse_delta: { x: number, y: number },\n");
    s.push_str("    mouse_wheel_delta: number,\n");
    // Method API (pass a `KeyCode` string, e.g. `"KeyA"`, or a mouse-button name, e.g. `"Left"`).
    s.push_str("    is_key_pressed: (self: ScriptInput, key: KeyCode) -> boolean,\n");
    s.push_str("    is_key_just_pressed: (self: ScriptInput, key: KeyCode) -> boolean,\n");
    s.push_str("    is_mouse_pressed: (self: ScriptInput, button: string) -> boolean,\n");
    s.push_str("    is_mouse_just_pressed: (self: ScriptInput, button: string) -> boolean,\n");
    s.push_str("    get_mouse_position: (self: ScriptInput) -> { x: number, y: number },\n");
    s.push_str("}\n\n");

    s.push_str("export type ScriptEvent = { name: string, x: number, y: number }\n\n");

    // Per-component types registered via `#[derive(Scriptable)]`.
    for t in iter::<ScriptType>() {
        s.push_str(t.type_def);
        s.push('\n');
    }
    s.push('\n');

    // The script context passed to `start(ctx)` / `update(ctx)`.
    s.push_str(
        "export type ScriptContext = {\n    entity: number,\n    dt: number,\n    input: ScriptInput,\n    components: { [string]: any },\n    events: { ScriptEvent },\n    events_in: { ScriptEvent },\n    GetComponent: <T>(self: ScriptContext, component: T) -> T,\n    HasComponent: <T>(self: ScriptContext, component: T) -> boolean,\n    AddComponent: <T>(self: ScriptContext, component: T, value: any) -> (),\n    RemoveComponent: <T>(self: ScriptContext, component: T) -> (),\n    Spawn: (self: ScriptContext, path: string) -> number,\n    Destroy: (self: ScriptContext, entity: number) -> (),\n}\n\n",
    );

    // Component "class" values, e.g. `Transform`, so `ctx:GetComponent(Transform)`
    // is typed. They are emitted as `local Name: Type = nil :: Type` (no `export`)
    // and exposed through the `return` block below, so a script that does
    // `local runa = require("runa")` gets `runa.Transform`, `runa.SpriteRenderer`, ...
    let comps: Vec<&str> = iter::<ScriptType>().map(|t| t.name).collect();
    for c in &comps {
        // `local Name: Type = {} :: Type` declares a typed, module-local value.
        // It is an empty table at runtime in this type-def file (the engine supplies
        // the real table via the runtime `require`). We initialize it with a `{} :: Type`
        // cast (empty table and the component type are both tables, so luau-lsp lets
        // the cast through) to silence `UninitializedLocal` without `?` or `export`.
        // The `return` below puts `Name` into the module's interface, so
        // `runa.Name` type-checks for scripts that do `local runa = require("runa")`.
        s.push_str(&format!("local {c}: {c} = {{}} :: {c};\n"));
    }
    s.push('\n');

    // The module must `return` exactly one value, otherwise luau-lsp rejects
    // `require("runa")` ("Module does not return exactly 1 value"). The values
    // reference the `local` declarations above (typed, nil in the type-def file —
    // the engine supplies the real tables via the runtime `require`).
    s.push_str("return {\n");
    for c in &comps {
        s.push_str(&format!("    {c} = {c},\n"));
    }
    // Math helpers (real impl is provided at runtime by the engine; the bodies here
    // only exist so luau-lsp can infer `runa.vec2` / `runa.normalize2` / ... types).
    s.push_str("    vec2 = function(x: number, y: number): Vec2 return { x = x, y = y } end,\n");
    s.push_str("    vec3 = function(x: number, y: number, z: number): Vec3 return { x = x, y = y, z = z } end,\n");
    s.push_str("    normalize2 = function(v: Vec2): Vec2 return v end,\n");
    s.push_str("    normalize3 = function(v: Vec3): Vec3 return v end,\n");
    s.push_str("    length2 = function(v: Vec2): number return 0 end,\n");
    s.push_str("    length3 = function(v: Vec3): number return 0 end,\n");
    // Scalar math helpers (real impl provided at runtime by the engine).
    s.push_str("    pi = 3.141592653589793,\n");
    s.push_str("    cos = function(x: number): number return 0 end,\n");
    s.push_str("    sin = function(x: number): number return 0 end,\n");
    s.push_str("    tan = function(x: number): number return 0 end,\n");
    s.push_str("    atan2 = function(y: number, x: number): number return 0 end,\n");
    s.push_str("    sqrt = function(x: number): number return 0 end,\n");
    s.push_str("    abs = function(x: number): number return 0 end,\n");
    s.push_str("    floor = function(x: number): number return 0 end,\n");
    s.push_str("    ceil = function(x: number): number return 0 end,\n");
    s.push_str("    round = function(x: number): number return 0 end,\n");
    s.push_str("    sign = function(x: number): number return 0 end,\n");
    s.push_str("    pow = function(x: number, y: number): number return 0 end,\n");
    s.push_str("    max = function(a: number, b: number): number return 0 end,\n");
    s.push_str("    min = function(a: number, b: number): number return 0 end,\n");
    s.push_str("    clamp = function(x: number, lo: number, hi: number): number return 0 end,\n");
    s.push_str("    rad = function(x: number): number return 0 end,\n");
    s.push_str("    deg = function(x: number): number return 0 end,\n");
    s.push_str("}\n");

    // `#[script_fn]` functions are registered at runtime onto the `runa` module
    // and as bare globals. List every discovered one so the LSP knows their names.
    s.push_str("\n-- #[script_fn] free Rust functions (registered at runtime as `runa.<name>` / `<name>`):\n");
    for func in iter::<ScriptFunction>() {
        s.push_str(&format!(
            "{} = function(...): any end\n",
            func.name
        ));
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, &s);
}

fn keycode_names() -> &'static [&'static str] {
    &[
        "KeyA",
        "KeyB",
        "KeyC",
        "KeyD",
        "KeyE",
        "KeyF",
        "KeyG",
        "KeyH",
        "KeyI",
        "KeyJ",
        "KeyK",
        "KeyL",
        "KeyM",
        "KeyN",
        "KeyO",
        "KeyP",
        "KeyQ",
        "KeyR",
        "KeyS",
        "KeyT",
        "KeyU",
        "KeyV",
        "KeyW",
        "KeyX",
        "KeyY",
        "KeyZ",
        "Digit0",
        "Digit1",
        "Digit2",
        "Digit3",
        "Digit4",
        "Digit5",
        "Digit6",
        "Digit7",
        "Digit8",
        "Digit9",
        "ArrowUp",
        "ArrowDown",
        "ArrowLeft",
        "ArrowRight",
        "Space",
        "Enter",
        "Escape",
        "Tab",
        "Backspace",
        "Delete",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "Insert",
        "ShiftLeft",
        "ShiftRight",
        "ControlLeft",
        "ControlRight",
        "AltLeft",
        "AltRight",
        "Meta",
        "SuperLeft",
        "SuperRight",
        "CapsLock",
        "F1",
        "F2",
        "F3",
        "F4",
        "F5",
        "F6",
        "F7",
        "F8",
        "F9",
        "F10",
        "F11",
        "F12",
        "Backquote",
        "Minus",
        "Equal",
        "BracketLeft",
        "BracketRight",
        "Backslash",
        "Semicolon",
        "Quote",
        "Comma",
        "Period",
        "Slash",
        "Numpad0",
        "Numpad1",
        "Numpad2",
        "Numpad3",
        "Numpad4",
        "Numpad5",
        "Numpad6",
        "Numpad7",
        "Numpad8",
        "Numpad9",
        "NumpadAdd",
        "NumpadSubtract",
        "NumpadMultiply",
        "NumpadDivide",
        "NumpadDecimal",
        "NumpadEnter",
    ]
}
