#![allow(clippy::wrong_self_convention)]
#![allow(clippy::needless_lifetimes)]

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use luau::{Function, Lua, Table, Value};
use runa_core::resources::event::{Event, EventBus};
use runa_core::resources::input::InputState;
use runa_core::resources::Time;
use runa_ecs::{Entity, World, R};
use runa_macros::system;
use runa_script_api::{iter, ScriptType};

/// Event emitted by scripts via `ctx.events[#ctx.events + 1] = {...}`.
#[derive(Debug, Clone)]
pub struct ScriptEvent {
    pub name: String,
    pub x: f32,
    pub y: f32,
}
impl Event for ScriptEvent {}

/// Per-world inbox that forwards engine `ScriptEvent`s into Lua. `script_system`
/// drains it once per frame into each script's `ctx.events_in` table. A single
/// `EventBus` subscriber (installed lazily per world, see `installed`) feeds it, so
/// Rust systems and other scripts can emit events that Lua scripts react to.
#[derive(Default)]
struct ScriptEventInbox {
    pub events: Arc<Mutex<Vec<ScriptEvent>>>,
    pub installed: bool,
}

/// Resolves the component name from the argument passed to `GetComponent` /
/// `HasComponent`: a plain string, or a component "class" table (carrying its
/// name under `__name`, as set on the Lua globals).
fn component_name(component: &Value) -> Option<String> {
    match component {
        Value::String(s) => s.to_str().ok().map(|st| st.to_string()),
        Value::Table(t) => t.get::<String>("__name").ok(),
        _ => None,
    }
}

/// Spawns a `ScriptComponent` whose `.luau` path is resolved relative to the
/// invoking crate's `CARGO_MANIFEST_DIR` (mirrors `load_image!`).
#[macro_export]
macro_rules! load_script {
    ($path:expr) => {{
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push($path);
        $crate::ScriptComponent::new(p.to_str().unwrap_or($path))
    }};
}

/// A scripted entity. Holds its own Luau VM instance and reloads the `.luau`
/// source when the file on disk changes (hot reload).
pub struct ScriptComponent {
    path: PathBuf,
    lua: Rc<Lua>,
    last_modified: Option<SystemTime>,
    started: bool,
}

impl ScriptComponent {
    pub fn new(path: &str) -> Self {
        let lua = Rc::new(Lua::new().expect("failed to create Luau VM"));
        // Register the `runa` module + component class globals on the VM *before*
        // the script is first loaded, so a top-level `require("runa")` resolves.
        setup_runa_module(&lua);
        Self {
            path: PathBuf::from(path),
            lua,
            last_modified: None,
            started: false,
        }
    }

    /// Re-executes the source if the file changed, redefining `start`/`update`.
    /// A script may `return { start = start, update = update }` so the engine picks
    /// up its callbacks from the returned table (and luau-lsp sees them as used);
    /// otherwise it falls back to the global `start`/`update` functions.
    pub fn reload_if_changed(&mut self) {
        if let Ok(meta) = fs::metadata(self.path.clone()) {
            if let Ok(mtime) = meta.modified() {
                if self.last_modified != Some(mtime) {
                    if let Ok(src) = fs::read_to_string(self.path.clone()) {
                        if let (Ok(Value::Table(tbl)), Ok(g)) = (
                            self.lua.load(src.as_str()).call::<Value>(()),
                            self.lua.globals(),
                        ) {
                            let _ = g.set("__runa_callbacks", tbl);
                        }
                        self.last_modified = Some(mtime);
                        self.started = false;
                    }
                }
            }
        }
    }
}

/// Builds the engine's `runa` module table (one marker table per `#[derive(Scriptable)]`
/// component, carrying its name under `__name`) and registers it so `require("runa")`
/// resolves at runtime. Also exposes each class as a bare global (`Transform`,
/// `SpriteRenderer`, ...) for convenience. Runs once per Lua VM, before any script
/// executes `require("runa")`.
fn setup_runa_module(lua: &Lua) {
    let globals = match lua.globals() {
        Ok(g) => g,
        Err(_) => return,
    };
    let runa = match lua.create_table() {
        Ok(t) => t,
        Err(_) => return,
    };
    for t in iter::<ScriptType>() {
        let class_tbl = match lua.create_table() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let _ = class_tbl.set("__name", t.name);
        let class_tbl_runa = match class_tbl.try_clone() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let _ = globals.set(t.name, class_tbl);
        let _ = runa.set(t.name, class_tbl_runa);
    }

    // Math helpers, backed by Rust (glam under the hood) — no Lua reimplementation
    // needed. They live on the `runa` module so scripts call `runa.normalize2(v)`, etc.
    let _ = runa.set(
        "vec2",
        lua.create_function(luau::callback!(|lua, x: f64, y: f64| {
            let t = lua.create_table()?;
            t.set("x", x)?;
            t.set("y", y)?;
            Ok(t)
        }))
        .expect("vec2"),
    );
    let _ = runa.set(
        "vec3",
        lua.create_function(luau::callback!(|lua, x: f64, y: f64, z: f64| {
            let t = lua.create_table()?;
            t.set("x", x)?;
            t.set("y", y)?;
            t.set("z", z)?;
            Ok(t)
        }))
        .expect("vec3"),
    );
    let _ = runa.set(
        "normalize2",
        lua.create_function(luau::callback!(|lua, v: Table| {
            let x: f64 = v.get("x")?;
            let y: f64 = v.get("y")?;
            let len = (x * x + y * y).sqrt();
            let t = lua.create_table()?;
            if len > 1e-8 {
                t.set("x", x / len)?;
                t.set("y", y / len)?;
            } else {
                t.set("x", 0.0)?;
                t.set("y", 0.0)?;
            }
            Ok(t)
        }))
        .expect("normalize2"),
    );
    let _ = runa.set(
        "normalize3",
        lua.create_function(luau::callback!(|lua, v: Table| {
            let x: f64 = v.get("x")?;
            let y: f64 = v.get("y")?;
            let z: f64 = v.get("z")?;
            let len = (x * x + y * y + z * z).sqrt();
            let t = lua.create_table()?;
            if len > 1e-8 {
                t.set("x", x / len)?;
                t.set("y", y / len)?;
                t.set("z", z / len)?;
            } else {
                t.set("x", 0.0)?;
                t.set("y", 0.0)?;
                t.set("z", 0.0)?;
            }
            Ok(t)
        }))
        .expect("normalize3"),
    );
    let _ = runa.set(
        "length2",
        lua.create_function(luau::callback!(|_lua, v: Table| {
            let x: f64 = v.get("x")?;
            let y: f64 = v.get("y")?;
            Ok((x * x + y * y).sqrt())
        }))
        .expect("length2"),
    );
    let _ = runa.set(
        "length3",
        lua.create_function(luau::callback!(|_lua, v: Table| {
            let x: f64 = v.get("x")?;
            let y: f64 = v.get("y")?;
            let z: f64 = v.get("z")?;
            Ok((x * x + y * y + z * z).sqrt())
        }))
        .expect("length3"),
    );

    // Cache the module table so a (custom) `require("runa")` can return it at runtime.
    let _ = globals.set("__runa_module", runa);

    // This VM does not ship `package`/`require` (it is not part of `StdLib::ALL`),
    // so install a minimal `require` that resolves the engine-provided `runa` module.
    // luau-lsp still type-checks `require("runa")` against the generated `runa.luau`,
    // which `return`s the module table.
    let require = lua
        .create_function(luau::callback!(|_lua, name: String| {
            let globals = _lua.globals()?;
            if name == "runa" {
                globals
                    .get::<Table>("__runa_module")
                    .map_err(|_| luau::Error::runtime("runa module not initialized"))
            } else {
                Err(luau::Error::runtime(format!(
                    "module '{name}' is not available"
                )))
            }
        }))
        .expect("create require");
    let _ = globals.set("require", require);
}

/// All winit `KeyCode` variant names, in the exact form produced by
/// `format!("{:?}", key_code)`. Centralized here so the Luau `KeyCode` union and
/// `ScriptInput` type are generated automatically — no hand-written list in Lua.
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

/// Regenerates the Luau type-definition module (`runa.luau`) that `luau-lsp`
/// reads for autocomplete / type checking. Call this from your app's startup
/// (once) with the directory your `.luau` scripts live in, e.g.
/// `write_luau_types(Path::new("examples/lua_scripting_test/scripts/runa.luau"))`.
///
/// Everything here is derived automatically:
/// - math types (`Vec3`, `Vec4`, `Vec2`, `Quat`),
/// - the `KeyCode` union + `ScriptInput` (from `keycode_names`),
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
    // Method API (pass a `KeyCode` string, e.g. `\"KeyA\"`, or a mouse-button name, e.g. `\"Left\"`).
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
        "export type ScriptContext = {\n    entity: number,\n    dt: number,\n    input: ScriptInput,\n    components: { [string]: any },\n    events: { ScriptEvent },\n    events_in: { ScriptEvent },\n    GetComponent: <T>(self: ScriptContext, component: T) -> T,\n    HasComponent: <T>(self: ScriptContext, component: T) -> boolean,\n}\n\n",
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
    s.push_str("}\n");

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, &s);
}

#[system(Update)]
pub fn script_system(world: &mut World) {
    let dt = world.get_resource::<Time>().delta as f64;

    // Lazily install a per-world `EventBus` subscriber that forwards engine-emitted
    // `ScriptEvent`s into the `ScriptEventInbox` resource, so Lua scripts can react to them.
    world.init_resource::<ScriptEventInbox>();
    let should_install = !world.get_resource::<ScriptEventInbox>().installed;
    if should_install {
        let arc = world.get_resource::<ScriptEventInbox>().events.clone();
        world
            .get_resource_mut::<EventBus>()
            .subscribe(move |ev: &ScriptEvent| {
                arc.lock().unwrap().push(ScriptEvent {
                    name: ev.name.clone(),
                    x: ev.x,
                    y: ev.y,
                });
            });
        world.get_resource_mut::<ScriptEventInbox>().installed = true;
    }
    // Dispatch queued events to subscribers (incl. the Lua forwarder) so the inbox
    // is populated before we drain it for this frame. `process()` drains the queue,
    // so a second call elsewhere in the frame is a no-op.
    world.get_resource_mut::<EventBus>().process();
    // Drain the inbox once per frame; every script receives the same snapshot.
    let incoming: Vec<ScriptEvent> = std::mem::take(
        &mut *world
            .get_resource::<ScriptEventInbox>()
            .events
            .lock()
            .unwrap(),
    );

    let entities: Vec<Entity> = world
        .query::<R<ScriptComponent>>()
        .map(|(e, _)| e)
        .collect();

    // Collect all scriptable types registered via `#[derive(Scriptable)]`.
    let types: Vec<&ScriptType> = iter::<ScriptType>().collect();

    for e in entities {
        if let Some(sc) = world.get_mut::<ScriptComponent>(e) {
            sc.reload_if_changed();
        }

        // Clone the VM handle (cheap) so we can keep `world` mutable later.
        let lua = match world.get::<ScriptComponent>(e) {
            Some(sc) => sc.lua.clone(),
            None => continue,
        };

        // Build the `ctx` table the script will read/mutate. Luau tables are
        // reference types, so storing it into globals / passing it to
        // `update(ctx)` keeps pointing at the SAME table the script mutates.
        let ctx = lua.create_table().expect("ctx");
        ctx.set("entity", e as i64).ok();
        ctx.set("dt", dt).ok();

        let input_tbl = lua.create_table().expect("input");
        let input = world.get_resource::<InputState>();
        // Convenience booleans: `ctx.input.KeyA == true` while held.
        for kc in &input.keys_pressed {
            input_tbl.set(format!("{:?}", kc), true).ok();
        }
        // `just_pressed` set (no top-level equivalent) -> `is_key_just_pressed`.
        let just = lua.create_table().expect("input.just");
        for kc in &input.keys_just_pressed {
            just.set(format!("{:?}", kc), true).ok();
        }
        input_tbl.set("keys_just_pressed", just).ok();
        // Mouse buttons (keyed by `Debug` name: "Left", "Right", ...).
        let mb_pressed = lua.create_table().expect("mb");
        for b in &input.mouse_buttons_pressed {
            mb_pressed.set(format!("{:?}", b), true).ok();
        }
        input_tbl.set("mouse_buttons_pressed", mb_pressed).ok();
        let mb_just = lua.create_table().expect("mbj");
        for b in &input.mouse_buttons_just_pressed {
            mb_just.set(format!("{:?}", b), true).ok();
        }
        input_tbl.set("mouse_buttons_just_pressed", mb_just).ok();
        // Mouse position / delta / wheel.
        let mp = lua.create_table().expect("mp");
        mp.set("x", input.mouse_position.0).ok();
        mp.set("y", input.mouse_position.1).ok();
        input_tbl.set("mouse_position", mp).ok();
        let md = lua.create_table().expect("md");
        md.set("x", input.mouse_delta.0).ok();
        md.set("y", input.mouse_delta.1).ok();
        input_tbl.set("mouse_delta", md).ok();
        input_tbl
            .set("mouse_wheel_delta", input.mouse_wheel_delta)
            .ok();
        // Method API. `is_key_pressed` reads the top-level booleans directly;
        // the rest read their respective sub-tables via `input_set_lookup`.
        input_tbl
            .set(
                "is_key_pressed",
                lua.create_function(luau::callback!(|_lua, self_tbl: Table, key: String| {
                    Ok(self_tbl.get::<bool>(key).unwrap_or(false))
                }))
                .expect("fn"),
            )
            .ok();
        input_tbl
            .set(
                "is_key_just_pressed",
                lua.create_function(luau::callback!(|_lua, self_tbl: Table, key: String| {
                    Ok(self_tbl
                        .get::<Table>("keys_just_pressed")
                        .ok()
                        .and_then(|t| t.get::<bool>(key).ok())
                        .unwrap_or(false))
                }))
                .expect("fn"),
            )
            .ok();
        input_tbl
            .set(
                "is_mouse_pressed",
                lua.create_function(luau::callback!(|_lua, self_tbl: Table, key: String| {
                    Ok(self_tbl
                        .get::<Table>("mouse_buttons_pressed")
                        .ok()
                        .and_then(|t| t.get::<bool>(key).ok())
                        .unwrap_or(false))
                }))
                .expect("fn"),
            )
            .ok();
        input_tbl
            .set(
                "is_mouse_just_pressed",
                lua.create_function(luau::callback!(|_lua, self_tbl: Table, key: String| {
                    Ok(self_tbl
                        .get::<Table>("mouse_buttons_just_pressed")
                        .ok()
                        .and_then(|t| t.get::<bool>(key).ok())
                        .unwrap_or(false))
                }))
                .expect("fn"),
            )
            .ok();
        input_tbl
            .set(
                "get_mouse_position",
                lua.create_function(luau::callback!(|lua, self_tbl: Table| {
                    Ok(self_tbl
                        .get::<Table>("mouse_position")
                        .unwrap_or_else(|_| lua.create_table().expect("mp")))
                }))
                .expect("fn"),
            )
            .ok();
        ctx.set("input", input_tbl).ok();

        let events = lua.create_table().expect("events");
        ctx.set("events", events).ok();

        // Incoming events forwarded from the engine this frame. Use `push` so the
        // entries land in the array part (sequential 1..n) — that keeps `#events_in`
        // and `ipairs(events_in)` working from Lua.
        let events_in = lua.create_table().expect("events_in");
        for ev in &incoming {
            let et = lua.create_table().expect("ev");
            et.set("name", ev.name.clone()).ok();
            et.set("x", ev.x).ok();
            et.set("y", ev.y).ok();
            events_in.push(et).ok();
        }
        ctx.set("events_in", events_in).ok();

        let comps = lua.create_table().expect("components");
        for t in &types {
            if let Some(tbl) = (t.to_luau)(&lua, world, e) {
                comps.set(t.name, tbl).ok();
            }
        }
        ctx.set("components", comps).ok();

        // Named binding (not a scrutinee temporary) so its drop is well-ordered
        // before `lua`.
        let globals_res = lua.globals();
        let globals = if let Ok(g) = globals_res {
            g
        } else {
            continue;
        };

        let get_component = lua
            .create_function(luau::callback!(|_lua, _self: Table, component: Value| {
                let name = match component_name(&component) {
                    Some(n) => n,
                    None => return Ok(Value::Nil),
                };
                let comps_r: Table = _self.get("components")?;
                let v: Value = comps_r.get(name)?;
                Ok(v)
            }))
            .expect("GetComponent");
        ctx.set("GetComponent", get_component).ok();

        let has_component = lua
            .create_function(luau::callback!(|_lua, _self: Table, component: Value| {
                let name = match component_name(&component) {
                    Some(n) => n,
                    None => return Ok(false),
                };
                let comps_r: Table = _self.get("components")?;
                let present = comps_r.contains_key(name)?;
                Ok(present)
            }))
            .expect("HasComponent");
        ctx.set("HasComponent", has_component).ok();

        // Run the script. `ctx` is passed as the argument, so the script sees it.
        let should_start = world
            .get::<ScriptComponent>(e)
            .map(|sc| !sc.started)
            .unwrap_or(false);
        let callbacks: Option<Table> = globals.get::<Table>("__runa_callbacks").ok();

        if should_start {
            let start_fn: Option<Function> = callbacks
                .as_ref()
                .and_then(|c| c.get::<Function>("start").ok())
                .or_else(|| globals.get::<Function>("start").ok());
            if let Some(f) = start_fn {
                let _ = f.call::<()>(&ctx);
            }
            if let Some(sc) = world.get_mut::<ScriptComponent>(e) {
                sc.started = true;
            }
        }
        let update_fn: Option<Function> = callbacks
            .as_ref()
            .and_then(|c| c.get::<Function>("update").ok())
            .or_else(|| globals.get::<Function>("update").ok());
        if let Some(f) = update_fn {
            let _ = f.call::<()>(&ctx);
        }

        // Apply-back using the SAME `ctx` table the script mutated.
        let comps_res = ctx.get::<Table>("components");
        if let Ok(comps) = comps_res {
            for t in &types {
                if let Ok(tbl) = comps.get::<Table>(t.name) {
                    (t.from_luau)(&lua, Value::Table(tbl), world, e);
                }
            }
        }
        let events_res = ctx.get::<Table>("events");
        if let Ok(events_tbl) = events_res {
            for (_i, ev) in events_tbl.pairs::<i64, Table>().flatten() {
                let name: String = ev.get("name").unwrap_or_default();
                let x: f32 = ev.get("x").unwrap_or(0.0) as f32;
                let y: f32 = ev.get("y").unwrap_or(0.0) as f32;
                world
                    .get_resource_mut::<EventBus>()
                    .emit(ScriptEvent { name, x, y });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runa_core::resources::event::EventBus;
    use runa_core::resources::input::InputState;

    #[test]
    fn lua_moves_transform() {
        let mut path = std::env::temp_dir();
        path.push("runa_test_player_move.luau");
        let src = r#"
            local runa = require("runa")
            local Transform = runa.Transform
            function start(ctx: runa.ScriptContext) end
            function update(ctx: runa.ScriptContext)
                local t = ctx:GetComponent(Transform)
                if t then t.position.x = t.position.x + 1.0 * ctx.dt end
            end
            return {
                start = start,
                update = update,
            }
        "#;
        std::fs::write(&path, src).unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<InputState>();
        world.init_resource::<EventBus>();
        let e = world.spawn((
            runa_core::components::Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));
        world.get_resource_mut::<Time>().delta = 0.5;

        script_system(&mut world);

        let x = world
            .get::<runa_core::components::Transform>(e)
            .unwrap()
            .position
            .x;
        assert!((x - 0.5).abs() < 1e-3, "expected x≈0.5, got {x}");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lua_input_api() {
        use winit::event::MouseButton;
        use winit::keyboard::KeyCode;

        let mut path = std::env::temp_dir();
        path.push("runa_test_input_api.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext) end
            function update(ctx: runa.ScriptContext)
                captured_key = ctx.input:is_key_pressed("KeyA")
                captured_just_true = ctx.input:is_key_just_pressed("KeyW")
                captured_just_false = ctx.input:is_key_just_pressed("KeyA")
                captured_mouse = ctx.input:is_mouse_pressed("Left")
                captured_mouse_just = ctx.input:is_mouse_just_pressed("Right")
                local mp = ctx.input:get_mouse_position()
                captured_mx = mp.x
                captured_my = mp.y
            end
            return { start = start, update = update }
        "#;
        std::fs::write(&path, src).unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<EventBus>();
        world.init_resource::<InputState>();
        let input = world.get_resource_mut::<InputState>();
        input.keys_pressed.insert(KeyCode::KeyA);
        input.keys_just_pressed.insert(KeyCode::KeyW);
        input.mouse_buttons_pressed.insert(MouseButton::Left);
        input.mouse_buttons_just_pressed.insert(MouseButton::Right);
        input.mouse_position = (12.0, 34.0);

        let e = world.spawn((
            runa_core::components::Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));
        world.get_resource_mut::<Time>().delta = 0.5;

        script_system(&mut world);

        let lua = &world.get::<ScriptComponent>(e).unwrap().lua;
        let g = lua.globals().expect("globals");
        assert!(g.get::<bool>("captured_key").unwrap());
        assert!(g.get::<bool>("captured_just_true").unwrap());
        assert!(!g.get::<bool>("captured_just_false").unwrap());
        assert!(g.get::<bool>("captured_mouse").unwrap());
        assert!(g.get::<bool>("captured_mouse_just").unwrap());
        assert!((g.get::<f64>("captured_mx").unwrap() - 12.0).abs() < 1e-3);
        assert!((g.get::<f64>("captured_my").unwrap() - 34.0).abs() < 1e-3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lua_math_helpers() {
        let mut path = std::env::temp_dir();
        path.push("runa_test_math.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext) end
            function update(ctx: runa.ScriptContext)
                local v = runa.vec2(3, 4)
                captured_len = runa.length2(v)
                local n = runa.normalize2(v)
                captured_nx = n.x
                captured_ny = n.y
                local z = runa.normalize2(runa.vec2(0, 0))
                captured_zx = z.x
                captured_zy = z.y
            end
            return { start = start, update = update }
        "#;
        std::fs::write(&path, src).unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<EventBus>();
        world.init_resource::<InputState>();
        let e = world.spawn((
            runa_core::components::Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));
        world.get_resource_mut::<Time>().delta = 0.5;
        script_system(&mut world);

        let lua = &world.get::<ScriptComponent>(e).unwrap().lua;
        let g = lua.globals().expect("globals");
        assert!((g.get::<f64>("captured_len").unwrap() - 5.0).abs() < 1e-6);
        assert!((g.get::<f64>("captured_nx").unwrap() - 0.6).abs() < 1e-6);
        assert!((g.get::<f64>("captured_ny").unwrap() - 0.8).abs() < 1e-6);
        assert!((g.get::<f64>("captured_zx").unwrap() - 0.0).abs() < 1e-6);
        assert!((g.get::<f64>("captured_zy").unwrap() - 0.0).abs() < 1e-6);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lua_emits_events() {
        use std::sync::{Arc, Mutex};

        // Collect emitted `ScriptEvent`s via an `EventBus` subscriber. The closure
        // must be `Send`, so we share state through `Arc<Mutex<_>>`.
        let captured = Arc::new(Mutex::new(Vec::<(String, f32, f32)>::new()));

        let mut path = std::env::temp_dir();
        path.push("runa_test_events.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                ctx.events[#ctx.events + 1] = { name = "spawn", x = 10, y = 20 }
            end
            function update(ctx: runa.ScriptContext)
                ctx.events[#ctx.events + 1] = { name = "tick", x = 1, y = 2 }
            end
            return { start = start, update = update }
        "#;
        std::fs::write(&path, src).unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<InputState>();
        world.init_resource::<EventBus>();
        {
            let sink = captured.clone();
            world
                .get_resource_mut::<EventBus>()
                .subscribe(move |e: &ScriptEvent| {
                    sink.lock().unwrap().push((e.name.clone(), e.x, e.y));
                });
        }

        world.spawn((
            runa_core::components::Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));
        world.get_resource_mut::<Time>().delta = 0.5;

        // Run one frame (start + update both emit) and dispatch queued events.
        script_system(&mut world);
        world.get_resource_mut::<EventBus>().process();

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 2, "expected start+update events");
        assert_eq!(events[0].0, "spawn");
        assert!((events[0].1 - 10.0).abs() < 1e-3);
        assert!((events[0].2 - 20.0).abs() < 1e-3);
        assert_eq!(events[1].0, "tick");
        assert!((events[1].1 - 1.0).abs() < 1e-3);
        assert!((events[1].2 - 2.0).abs() < 1e-3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn player_move_script_emits_events() {
        // Loads the actual demo script (`examples/.../player_move.luau`) and verifies
        // it emits events through the engine `EventBus` (so the demo's events are real,
        // not just dead code in the file).
        use std::sync::{Arc, Mutex};

        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../../examples/lua_scripting_test/scripts/player_move.luau");
        assert!(path.exists(), "player_move.luau not found at {path:?}");

        let captured = Arc::new(Mutex::new(Vec::<String>::new()));

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<InputState>();
        world.init_resource::<EventBus>();
        {
            let sink = captured.clone();
            world
                .get_resource_mut::<EventBus>()
                .subscribe(move |e: &ScriptEvent| {
                    sink.lock().unwrap().push(e.name.clone());
                });
        }

        let e = world.spawn((
            runa_core::components::Transform::default(),
            runa_core::components::SpriteRenderer::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));
        world.get_resource_mut::<Time>().delta = 0.5;

        script_system(&mut world);
        world.get_resource_mut::<EventBus>().process();

        let cap = captured.lock().unwrap();
        assert!(
            cap.iter().any(|n| n == "player_started"),
            "player_move.luau should emit `player_started` on start, got {cap:?}"
        );

        let _ = world.get::<ScriptComponent>(e);
    }

    #[test]
    fn lua_event_roundtrip() {
        use std::sync::{Arc, Mutex};

        // Captures every `ScriptEvent` that reaches the bus (including ones a script
        // re-emits), proving engine -> Lua -> engine delivery.
        let captured = Arc::new(Mutex::new(Vec::<String>::new()));

        let mut path = std::env::temp_dir();
        path.push("runa_test_event_rt.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext) end
            function update(ctx: runa.ScriptContext)
                for _i, ev in ipairs(ctx.events_in) do
                    received_name = ev.name
                    received_x = ev.x
                    ctx.events[#ctx.events + 1] = { name = "echo:" .. ev.name, x = ev.x, y = ev.y }
                end
            end
            return { start = start, update = update }
        "#;
        std::fs::write(&path, src).unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<InputState>();
        world.init_resource::<EventBus>();
        {
            let sink = captured.clone();
            world
                .get_resource_mut::<EventBus>()
                .subscribe(move |e: &ScriptEvent| {
                    sink.lock().unwrap().push(e.name.clone());
                });
        }

        let e = world.spawn((
            runa_core::components::Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));
        world.get_resource_mut::<Time>().delta = 0.5;

        // Frame 1: installs the Lua event forwarder; `events_in` is still empty.
        script_system(&mut world);
        // Engine emits an event that Lua should receive next frame.
        world.get_resource_mut::<EventBus>().emit(ScriptEvent {
            name: "ping".into(),
            x: 5.0,
            y: 6.0,
        });
        // Frame 2: Lua reads "ping" from `ctx.events_in` and re-emits "echo:ping".
        script_system(&mut world);
        world.get_resource_mut::<EventBus>().process();

        let lua = &world.get::<ScriptComponent>(e).unwrap().lua;
        let g = lua.globals().expect("globals");
        assert_eq!(g.get::<String>("received_name").unwrap(), "ping");
        assert!((g.get::<f64>("received_x").unwrap() - 5.0).abs() < 1e-3);
        let cap = captured.lock().unwrap();
        assert!(
            cap.iter().any(|n| n == "echo:ping"),
            "lua should have re-emitted echo:ping, got {cap:?}"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_types_check() {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../examples/lua_scripting_test/scripts/runa.luau");
        write_luau_types(&p);
        println!("wrote {}", p.display());
    }
}
