#![allow(clippy::wrong_self_convention)]
#![allow(clippy::needless_lifetimes)]

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use luau::{Function, Lua, Table, Value, Variadic};
use runa_core::resources::event::{Event, EventBus};
use runa_core::resources::input::InputState;
use runa_core::resources::Time;
use runa_ecs::{Entity, World, R};
use runa_macros::system;
use runa_script_api::{iter, ScriptFunction, ScriptType};

// `write_luau_types` lives in `runa_script_api` (so `runa_app` can call it without
// a `runa_engine -> runa_app` cycle); re-export it here so the public path
// `runa_engine::scripting::write_luau_types` keeps working.
pub use runa_script_api::write_luau_types;

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
        $crate::scripting::ScriptComponent::new(p.to_str().unwrap_or($path))
    }};
}

// Re-export the `#[macro_export]` macro into this module so it is reachable as
// `runa_engine::scripting::load_script` as well as the crate-root `runa_engine::load_script`.
pub use crate::load_script;

/// A scripted entity. Holds its own Luau VM instance and reloads the `.luau`
/// sources when any file on disk changes (hot reload). An entity may run several
/// scripts at once (multi-script), which is how engine constructors compose
/// behaviour from multiple small Luau files.
pub struct ScriptComponent {
    scripts: Vec<PathBuf>,
    lua: Rc<Lua>,
    last_modified: Vec<Option<SystemTime>>,
    started: bool,
}

impl ScriptComponent {
    /// Create a scripted entity that runs a single `.luau` file.
    pub fn new(path: &str) -> Self {
        Self::with_scripts(vec![PathBuf::from(path)])
    }

    /// Create a scripted entity that runs several `.luau` files (in order).
    pub fn with_scripts(scripts: Vec<PathBuf>) -> Self {
        let lua = Rc::new(Lua::new().expect("failed to create Luau VM"));
        // Register the `runa` module + component class globals on the VM *before*
        // the script is first loaded, so a top-level `require("runa")` resolves.
        setup_runa_module(&lua);
        Self {
            scripts,
            lua,
            last_modified: Vec::new(),
            started: false,
        }
    }

    /// Re-executes any changed source files, redefining their `start`/`update`
    /// callbacks. Each script may `return { start = start, update = update }` so the
    /// engine picks up its callbacks from the returned table (and luau-lsp sees them
    /// as used); otherwise it falls back to the global `start`/`update` functions.
    /// All callbacks live in the `__runa_scripts` array on the VM globals, one slot
    /// per script file, in `scripts` order.
    pub fn reload_if_changed(&mut self) {
        if self.last_modified.len() != self.scripts.len() {
            self.last_modified = vec![None; self.scripts.len()];
        }
        let lua = &self.lua;
        let g = match lua.globals() {
            Ok(g) => g,
            Err(_) => return,
        };
        // Ensure the `__runa_scripts` array exists; reuse it directly so we don't
        // depend on a round-trip `set`+`get` succeeding.
        let arr = match g.get::<Table>("__runa_scripts") {
            Ok(a) => a,
            Err(_) => {
                let t = lua.create_table().expect("scripts");
                let _ = g.set("__runa_scripts", t.try_clone().expect("scripts"));
                t
            }
        };
        for (i, path) in self.scripts.iter().enumerate() {
            if let Ok(meta) = fs::metadata(path) {
                if let Ok(mtime) = meta.modified() {
                    if self.last_modified[i] != Some(mtime) {
                        if let Ok(src) = fs::read_to_string(path) {
                            match lua.load(src.as_str()).call::<Value>(()) {
                                Ok(Value::Table(tbl)) => {
                                    let _ = arr.set((i as i64) + 1, tbl);
                                }
                                Ok(_) => {} // script defines globals instead of returning a table
                                Err(err) => {
                                    eprintln!("[script] load error in {}: {err}", path.display());
                                }
                            }
                            // Track the mtime even on failure so a syntax error doesn't
                            // re-trigger compilation every frame; fixing the file updates
                            // the mtime and reloads it.
                            self.last_modified[i] = Some(mtime);
                            self.started = false;
                        }
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
        "vec4",
        lua.create_function(luau::callback!(|lua, x: f64, y: f64, z: f64, w: f64| {
            let t = lua.create_table()?;
            t.set("x", x)?;
            t.set("y", y)?;
            t.set("z", z)?;
            t.set("w", w)?;
            Ok(t)
        }))
        .expect("vec4"),
    );
    let _ = runa.set(
        "quat",
        lua.create_function(luau::callback!(|lua, x: f64, y: f64, z: f64, w: f64| {
            let t = lua.create_table()?;
            t.set("x", x)?;
            t.set("y", y)?;
            t.set("z", z)?;
            t.set("w", w)?;
            Ok(t)
        }))
        .expect("quat"),
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

    // Scalar math helpers on the `runa` module so scripts can call `runa.cos(x)` etc.
    // (Luau's own `math` library is also available via `math.cos`, etc.)
    let _ = runa.set("pi", std::f64::consts::PI);
    let _ = runa.set(
        "cos",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.cos()) }))
            .expect("cos"),
    );
    let _ = runa.set(
        "sin",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.sin()) }))
            .expect("sin"),
    );
    let _ = runa.set(
        "tan",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.tan()) }))
            .expect("tan"),
    );
    let _ = runa.set(
        "atan2",
        lua.create_function(luau::callback!(|_lua, y: f64, x: f64| { Ok(y.atan2(x)) }))
            .expect("atan2"),
    );
    let _ = runa.set(
        "sqrt",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.sqrt()) }))
            .expect("sqrt"),
    );
    let _ = runa.set(
        "abs",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.abs()) }))
            .expect("abs"),
    );
    let _ = runa.set(
        "floor",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.floor()) }))
            .expect("floor"),
    );
    let _ = runa.set(
        "ceil",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.ceil()) }))
            .expect("ceil"),
    );
    let _ = runa.set(
        "round",
        lua.create_function(luau::callback!(|_lua, x: f64| { Ok(x.round()) }))
            .expect("round"),
    );
    let _ = runa.set(
        "sign",
        lua.create_function(luau::callback!(|_lua, x: f64| {
            Ok(if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            })
        }))
        .expect("sign"),
    );
    let _ = runa.set(
        "pow",
        lua.create_function(luau::callback!(|_lua, x: f64, y: f64| { Ok(x.powf(y)) }))
            .expect("pow"),
    );
    let _ = runa.set(
        "max",
        lua.create_function(luau::callback!(|_lua, a: f64, b: f64| { Ok(a.max(b)) }))
            .expect("max"),
    );
    let _ = runa.set(
        "min",
        lua.create_function(luau::callback!(|_lua, a: f64, b: f64| { Ok(a.min(b)) }))
            .expect("min"),
    );
    let _ = runa.set(
        "clamp",
        lua.create_function(luau::callback!(|_lua, x: f64, lo: f64, hi: f64| {
            Ok(x.max(lo).min(hi))
        }))
        .expect("clamp"),
    );
    let _ = runa.set(
        "rad",
        lua.create_function(luau::callback!(|_lua, x: f64| {
            Ok(x * std::f64::consts::PI / 180.0)
        }))
        .expect("rad"),
    );
    let _ = runa.set(
        "deg",
        lua.create_function(luau::callback!(|_lua, x: f64| {
            Ok(x * 180.0 / std::f64::consts::PI)
        }))
        .expect("deg"),
    );

    // Register all `#[script_fn]` functions: both on the `runa` module and as bare
    // globals, so scripts can call `runa.my_func(...)` or `my_func(...)`.
    for f in iter::<ScriptFunction>() {
        let func = f.func;
        let lf = match lua.create_function(luau::callback!(move |lua, args: Variadic<Value>| {
            func(lua, args)
        })) {
            Ok(lf) => lf,
            Err(_) => continue,
        };
        let lf_global =
            match lua.create_function(luau::callback!(move |lua, args: Variadic<Value>| {
                func(lua, args)
            })) {
                Ok(lf) => lf,
                Err(_) => continue,
            };
        let _ = runa.set(f.name, lf);
        let _ = globals.set(f.name, lf_global);
    }

    // GDScript-style convenience constructors so Luau scripts can build sprite data
    // ergonomically. Each returns a plain table matching the corresponding Rust
    // struct's field names, ready to pass to `ctx:AddComponent` / assign to a field.
    let _ = runa.set(
        "sprite_sheet",
        lua.create_function(luau::callback!(|lua, columns: u32, rows: u32| {
            let t = lua.create_table()?;
            t.set("columns", columns)?;
            t.set("rows", rows)?;
            Ok(t)
        }))
        .expect("sprite_sheet"),
    );
    let _ = runa.set(
        "sprite_clip",
        lua.create_function(luau::callback!(
            |lua, name: String, start_frame: u32, end_frame: u32, fps: f32, looping: bool| {
                let t = lua.create_table()?;
                t.set("name", name)?;
                t.set("start_frame", start_frame)?;
                t.set("end_frame", end_frame)?;
                t.set("fps", fps)?;
                t.set("looping", looping)?;
                Ok(t)
            }
        ))
        .expect("sprite_clip"),
    );
    let _ = runa.set(
        "sprite",
        lua.create_function(luau::callback!(|lua, path: String| {
            let t = lua.create_table()?;
            t.set("texture_path", path)?;
            Ok(t)
        }))
        .expect("sprite"),
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

#[system(Update, "crate")]
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

    // Deferred spawn/destroy requests, filled by `ctx:Spawn` / `ctx:Destroy` while
    // scripts run and drained below (outside any Lua callback).
    let spawn_queue: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let destroy_queue: Rc<RefCell<Vec<Entity>>> = Rc::new(RefCell::new(Vec::new()));

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
            if let Some(tbl) = (t.to_luau)((*lua).as_ref(), world, e) {
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

        // The world is only borrowed for the duration of `script_system`; these
        // closures capture a raw pointer to it (valid because they run synchronously
        // within this call) and are `'static`, as `create_function` requires.
        let world_ptr = world as *mut World;
        let types_for_calls: Vec<&'static ScriptType> = types.clone();
        let add_types = types_for_calls.clone();
        let rem_types = types_for_calls.clone();

        let add_component = lua
            .create_function(luau::callback!(
                move |lua, _self: Table, component: Value, value: Value| {
                    let world = unsafe { &mut *world_ptr };
                    if let Some(name) = component_name(&component) {
                        if let Some(t) = add_types.iter().find(|t| t.name == name) {
                            (t.add)(lua, value, world, e);
                        }
                    }
                    Ok(())
                }
            ))
            .expect("AddComponent");
        ctx.set("AddComponent", add_component).ok();

        let remove_component = lua
            .create_function(luau::callback!(
                move |_lua, _self: Table, component: Value| {
                    let world = unsafe { &mut *world_ptr };
                    if let Some(name) = component_name(&component) {
                        if let Some(t) = rem_types.iter().find(|t| t.name == name) {
                            (t.remove)(world, e);
                        }
                    }
                    Ok(())
                }
            ))
            .expect("RemoveComponent");
        ctx.set("RemoveComponent", remove_component).ok();

        // `Spawn`/`Destroy` are deferred: the Lua callbacks only enqueue requests,
        // and we process them *after* all scripts have run (outside any Lua
        // callback). This avoids creating/destroying a Luau VM — or mutating the
        // world's archetypes — re-entrantly while a script is executing, which
        // would panic.
        let spawn_queue = spawn_queue.clone();
        let spawn_fn = lua
            .create_function(luau::callback!(move |_lua, _self: Table, path: String| {
                spawn_queue.borrow_mut().push(PathBuf::from(path));
                Ok(())
            }))
            .expect("Spawn");
        ctx.set("Spawn", spawn_fn).ok();

        let destroy_queue = destroy_queue.clone();
        let destroy_fn = lua
            .create_function(luau::callback!(
                move |_lua, _self: Table, entity_id: i64| {
                    destroy_queue.borrow_mut().push(entity_id as u64);
                    Ok(())
                }
            ))
            .expect("Destroy");
        ctx.set("Destroy", destroy_fn).ok();

        // Run the scripts. `ctx` is passed as the argument, so the script sees it.
        let should_start = world
            .get::<ScriptComponent>(e)
            .map(|sc| !sc.started)
            .unwrap_or(false);

        let scripts_tbl: Table = globals
            .get("__runa_scripts")
            .unwrap_or_else(|_| lua.create_table().expect("scripts"));

        if should_start {
            let mut ran = false;
            for (_i, callbacks) in scripts_tbl.pairs::<i64, Table>().flatten() {
                if let Ok(f) = callbacks.get::<Function>("start") {
                    if let Err(err) = f.call::<()>(&ctx) {
                        eprintln!("[script] entity {e} script error: {err}");
                    }
                    ran = true;
                }
            }
            // Fall back to a top-level `start` for scripts that don't `return` a
            // callbacks table (e.g. the unit tests).
            if !ran {
                if let Ok(f) = globals.get::<Function>("start") {
                    if let Err(err) = f.call::<()>(&ctx) {
                        eprintln!("[script] entity {e} script error: {err}");
                    }
                }
            }
            if let Some(sc) = world.get_mut::<ScriptComponent>(e) {
                sc.started = true;
            }
        }
        let mut ran = false;
        for (_i, callbacks) in scripts_tbl.pairs::<i64, Table>().flatten() {
            if let Ok(f) = callbacks.get::<Function>("update") {
                if let Err(err) = f.call::<()>(&ctx) {
                    eprintln!("[script] entity {e} script error: {err}");
                }
                ran = true;
            }
        }
        if !ran {
            if let Ok(f) = globals.get::<Function>("update") {
                if let Err(err) = f.call::<()>(&ctx) {
                    eprintln!("[script] entity {e} script error: {err}");
                }
            }
        }

        // Apply-back using the SAME `ctx` table the script mutated.
        let comps_res = ctx.get::<Table>("components");
        if let Ok(comps) = comps_res {
            for t in &types {
                if let Ok(tbl) = comps.get::<Table>(t.name) {
                    (t.from_luau)((*lua).as_ref(), Value::Table(tbl), world, e);
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

    // Drain deferred spawn/destroy requests outside any Lua callback.
    for path in spawn_queue.borrow_mut().drain(..) {
        world.spawn((ScriptComponent::new(path.to_str().unwrap_or("")),));
    }
    for e in destroy_queue.borrow_mut().drain(..) {
        world.despawn(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runa_core::resources::event::EventBus;
    use runa_core::resources::input::InputState;
    use runa_macros::Scriptable;

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
        use runa_core::KeyCode;
        use runa_core::MouseButton;

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
        p.push("../../examples/lua_scripting_test/.runa/runa.luau");
        write_luau_types(&p);
        println!("wrote {}", p.display());
        let content = std::fs::read_to_string(&p).expect("read generated runa.luau");
        // Built-in engine types should come from the committed `runa_base.luau`.
        assert!(content.contains("export type Transform"));
        assert!(content.contains("export type Collider2DShape"));
    }

    // A user-defined, scriptable, addable component used to exercise runtime
    // `AddComponent` / `RemoveComponent`. Living inside `runa_engine`, it uses the
    // internal crate path; `addable` is the default so only `crate` is needed.
    #[derive(Debug, Clone, Default, Scriptable)]
    #[script(crate = "::runa_script_api")]
    struct Health {
        value: f32,
    }

    #[test]
    fn lua_add_and_get_component() {
        let mut path = std::env::temp_dir();
        path.push("runa_test_add_component.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                ctx:AddComponent(Health, { value = 42 })
            end
            function update(ctx: runa.ScriptContext)
                local h = ctx:GetComponent(Health)
                if h then captured = h.value end
            end
            return { start = start, update = update }
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

        // Frame 1: `start` adds the component. Frame 2: `update` can see it.
        script_system(&mut world);
        script_system(&mut world);

        let h = world
            .get::<Health>(e)
            .expect("Health should have been added");
        assert!((h.value - 42.0).abs() < 1e-6, "got {}", h.value);
        let lua = &world.get::<ScriptComponent>(e).unwrap().lua;
        assert!((lua.globals().expect("g").get::<f64>("captured").unwrap() - 42.0).abs() < 1e-6);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lua_remove_component() {
        let mut path = std::env::temp_dir();
        path.push("runa_test_remove_component.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                ctx:AddComponent(Health, { value = 1 })
            end
            function update(ctx: runa.ScriptContext)
                ctx:RemoveComponent(Health)
            end
            return { start = start, update = update }
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

        // `start` (add) and `update` (remove) both run within the first frame, so
        // after a single `script_system` call the component should be gone.
        script_system(&mut world);
        assert!(
            world.get::<Health>(e).is_none(),
            "Health should have been removed"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lua_spawn_entity() {
        let mut path = std::env::temp_dir();
        path.push("runa_test_spawn.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                ctx:Spawn("does_not_exist.luau")
            end
            function update(ctx: runa.ScriptContext) end
            return { start = start, update = update }
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

        script_system(&mut world);

        let _ = &world.get::<ScriptComponent>(e).unwrap().lua;

        // `ctx:Spawn` is deferred, so after the frame a second entity exists with
        // its own `ScriptComponent`.
        assert_eq!(world.query::<R<ScriptComponent>>().count(), 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lua_scalar_math() {
        let mut path = std::env::temp_dir();
        path.push("runa_test_scalar_math.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                cos_zero = runa.cos(0)
                sin_half_pi = runa.sin(runa.pi / 2)
                sqrt_16 = runa.sqrt(16)
                clamped = runa.clamp(5, 0, 3)
                lib_cos = math.cos(0)
                lib_pi = math.pi
            end
            function update(ctx: runa.ScriptContext) end
            return { start = start, update = update }
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

        script_system(&mut world);

        let lua = &world.get::<ScriptComponent>(e).unwrap().lua;
        let g = lua.globals().expect("g");
        assert!((g.get::<f64>("cos_zero").unwrap() - 1.0).abs() < 1e-9);
        assert!((g.get::<f64>("sin_half_pi").unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(g.get::<f64>("sqrt_16").unwrap(), 4.0);
        assert_eq!(g.get::<f64>("clamped").unwrap(), 3.0);
        assert!((g.get::<f64>("lib_cos").unwrap() - 1.0).abs() < 1e-9);
        assert!((g.get::<f64>("lib_pi").unwrap() - std::f64::consts::PI).abs() < 1e-9);

        let _ = std::fs::remove_file(&path);
    }
}
