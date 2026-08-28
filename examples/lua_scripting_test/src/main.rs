use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::asset::load_image;
use runa_engine::core::components::*;
use runa_engine::core::Vec3;
use runa_engine::ecs::World;
use runa_engine::macros::Scriptable;
use runa_engine::script_fn;
use runa_engine::scripting::load_script;

fn main() {
    // (debug) regenerate `scripts/runa.luau` from THIS binary so `luau-lsp` sees the
    // `#[script_fn]` functions defined in this crate (e.g. `add_scores`). `inventory`
    // only enumerates fns linked into the running binary, so we must generate from
    // here, not from `runa_app` (which doesn't link this example crate).
    #[cfg(debug_assertions)]
    {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let p = std::path::Path::new(&manifest).join("scripts/runa.luau");
        runa_engine::scripting::write_luau_types(&p);
    }

    let mut world = World::new();

    world.spawn((Camera::new_orthographic(32.0, 18.0),));

    world.spawn((
        Transform {
            scale: Vec3::new(1., 1., 16.),
            ..Default::default()
        },
        SpriteRenderer::new(Some(load_image!("assets/Charactert.png"))),
        load_script!("scripts/player_move.luau"),
    ));

    // Disable `runa_app`'s own (CWD-relative) `runa.luau` regeneration so it doesn't
    // overwrite the file this example just wrote above.
    let config = RunaWindowConfig {
        title: "Lua Scripting Test".to_string(),
        width: 1280,
        height: 720,
        vsync: false,
        show_fps_in_title: true,
        luau_types_path: None,
        ..Default::default()
    };

    let _ = RunaApp::run_with_config(world, config);
}

// A unit component (default `::runa_script_api` path — works because `runa_script_api`
// is a transitive dependency available by name).

// A user-defined component written in the *game* crate (which depends only on
// `runa_engine`). Because `addable` is now the default, a plain `#[derive(Scriptable)]`
// is already addable in Lua via `ctx:AddComponent(Speed, { value = 42 })`.
#[derive(Debug, Clone, Default, Scriptable)]
#[script(addable)]
struct Speed {
    value: f32,
}

// A free Rust function exposed to Luau via `#[script_fn]`. Scripts can call it as
// `runa.add_scores(a, b)` or just `add_scores(a, b)`.
#[script_fn]
fn add_scores(a: f32, b: f32) -> f32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    use runa_engine::core::resources::event::EventBus;
    use runa_engine::core::resources::input::InputState;
    use runa_engine::core::resources::Time;
    use runa_engine::scripting::{script_system, ScriptComponent};

    #[test]
    fn lua_calls_rust_function() {
        let dir = std::env::temp_dir();
        let mut p1 = dir.clone();
        p1.push("lua_rust_fn1.luau");
        std::fs::write(
            &p1,
            r#"
                local runa = require("runa")
                function start(ctx: runa.ScriptContext)
                    ctx:AddComponent(Speed, { value = runa.add_scores(2, 3) })
                end
                function update(ctx: runa.ScriptContext) end
                return { start = start, update = update }
            "#,
        )
        .unwrap();

        let mut p2 = dir.clone();
        p2.push("lua_rust_fn2.luau");
        std::fs::write(
            &p2,
            r#"
                function start(ctx: runa.ScriptContext)
                    ctx:AddComponent(Speed, { value = add_scores(10, 20) })
                end
                function update(ctx: runa.ScriptContext) end
                return { start = start, update = update }
            "#,
        )
        .unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<InputState>();
        world.init_resource::<EventBus>();
        let e1 = world.spawn((
            Transform::default(),
            ScriptComponent::new(p1.to_str().unwrap()),
        ));
        let e2 = world.spawn((
            Transform::default(),
            ScriptComponent::new(p2.to_str().unwrap()),
        ));

        script_system(&mut world);
        let s1 = world.get::<Speed>(e1).expect("module-path Speed");
        assert!((s1.value - 5.0).abs() < 1e-9);
        let s2 = world.get::<Speed>(e2).expect("global-path Speed");
        assert!((s2.value - 30.0).abs() < 1e-9);

        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
    }

    #[test]
    fn lua_add_external_component() {
        let mut path = std::env::temp_dir();
        path.push("lua_ext_comp.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                ctx:AddComponent(Speed, { value = 42 })
            end
            function update(ctx: runa.ScriptContext)
                local s = ctx:GetComponent(Speed)
                captured = s.value
            end
            return { start = start, update = update }
        "#;
        std::fs::write(&path, src).unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<InputState>();
        world.init_resource::<EventBus>();
        let e = world.spawn((
            Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));

        // `start` (add) and `update` (get) both run within the first frame.
        script_system(&mut world);
        let s = world
            .get::<Speed>(e)
            .expect("Speed should have been added by the script");
        assert_eq!(s.value, 42.0);

        let _ = std::fs::remove_file(&path);
    }

    // Regenerates `scripts/runa.luau` for this example (mirrors what `main` does in
    // debug) and asserts that `#[script_fn]` fns from THIS crate are enumerated.
    #[test]
    fn generate_example_runa_luau() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let p = std::path::Path::new(&manifest).join("scripts/runa.luau");
        runa_engine::scripting::write_luau_types(&p);

        let content = std::fs::read_to_string(&p).expect("read generated runa.luau");
        assert!(
            content.contains("add_scores = function"),
            "add_scores should appear in generated runa.luau (got:\n{content})"
        );
    }

    // ---- Item 1: Sprite loading + every SpriteRenderer field editable from Luau ----
    #[test]
    fn lua_sprite_renderer_editable() {
        // Absolute path so the runtime `TextureAsset::load` can actually read the file.
        let asset = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/Charactert.png");
        let mut path = std::env::temp_dir();
        path.push("lua_sprite_renderer.luau");
        let src = format!(
            r#"
                local runa = require("runa")
                function start(ctx: runa.ScriptContext)
                    ctx:AddComponent(SpriteRenderer, {{
                        texture_path = "{asset}",
                        pixels_per_unit = 32,
                        uv_rect = {{0, 0, 1, 1}},
                        color = {{1, 0.5, 0.25, 1}},
                        replace_color = true,
                        flip_x = true,
                        flip_y = false,
                    }})
                end
                function update(ctx: runa.ScriptContext) end
                return {{ start = start, update = update }}
            "#
        );
        std::fs::write(&path, src).unwrap();

        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<InputState>();
        world.init_resource::<EventBus>();
        let e = world.spawn((
            Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));

        script_system(&mut world);
        let sr = world.get::<SpriteRenderer>(e).expect("SpriteRenderer added");
        assert_eq!(sr.texture_path.as_deref(), Some(asset));
        assert!((sr.pixels_per_unit - 32.0).abs() < 1e-6);
        assert_eq!(sr.uv_rect, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(sr.color, [1.0, 0.5, 0.25, 1.0]);
        assert!(sr.replace_color);
        assert!(sr.flip_x);
        assert!(!sr.flip_y);
        // The sprite must genuinely load from the runtime path.
        assert!(sr.texture().is_some(), "texture should load from texture_path");

        let _ = std::fs::remove_file(&path);
    }

    // ---- Item 2: SpriteAnimator + convenient clip/sheet creation + switching ----
    #[test]
    fn lua_sprite_animator_editable() {
        let mut path = std::env::temp_dir();
        path.push("lua_sprite_animator.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                local sheet = runa.sprite_sheet(4, 2)
                local clipA = runa.sprite_clip("idle", 0, 3, 8, true)
                local clipB = runa.sprite_clip("run", 4, 7, 12, false)
                ctx:AddComponent(SpriteAnimator, {
                    sheet = sheet,
                    clips = { clipA, clipB },
                    current_clip = "run",
                    current_frame = 5,
                    playing = false,
                })
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
            Transform::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));

        script_system(&mut world);
        let a = world.get::<SpriteAnimator>(e).expect("SpriteAnimator added");
        assert_eq!(a.sheet.columns, 4);
        assert_eq!(a.sheet.rows, 2);
        assert_eq!(a.clips.len(), 2);
        assert_eq!(a.clips[0].name, "idle");
        assert_eq!(a.clips[1].name, "run");
        // "run" is the active clip → switching animations works from Luau.
        assert_eq!(a.current_clip.as_deref(), Some("run"));
        assert_eq!(a.current_frame, 5);
        assert!(!a.playing);

        let _ = std::fs::remove_file(&path);
    }

    // ---- Item 3: every other (non-broken) component's params editable from Luau ----
    #[test]
    fn lua_other_components_editable() {
        use runa_engine::core::{Quat, Vec2};

        let mut path = std::env::temp_dir();
        path.push("lua_components.luau");
        let src = r#"
            local runa = require("runa")
            function start(ctx: runa.ScriptContext)
                local t = ctx:GetComponent(Transform)
                t.position = runa.vec3(1, 2, 3)
                t.rotation = runa.vec4(0, 0, 0, 1)
                t.scale = runa.vec3(2, 2, 2)

                local c2 = ctx:GetComponent(Collider2D)
                c2.shape = { type = "Circle", radius = 2.5 }
                c2.offset = runa.vec2(1, 1)
                c2.enabled = false
                c2.is_trigger = true
                c2.layer = 7

                local c3 = ctx:GetComponent(Collider3D)
                c3.shape = { type = "Box", half_size = runa.vec3(1, 2, 3) }
                c3.offset = runa.vec3(0, 0, 5)
                c3.enabled = false
                c3.is_trigger = true
                c3.layer = 9

                local pc = ctx:GetComponent(PhysicsCollision)
                pc.size = runa.vec2(10, 20)
                pc.enabled = false

                local s = ctx:GetComponent(Sorting)
                s.order = 42
                s.y_sort = true
                s.y_offset = 3.5

                local cam = ctx:GetComponent(Camera)
                cam.position = runa.vec3(5, 6, 7)
                cam.target = runa.vec3(0, 0, 0)
                cam.up = runa.vec3(0, 1, 0)
                cam.projection = "Perspective"
                cam.orthographic_size = runa.vec2(40, 30)
                cam.near = 0.5
                cam.far = 2000
                cam.fov = 1.2

                local a = ctx:GetComponent(AudioSource)
                a.source_path = "sfx.wav"
                a.volume = 0.25
                a.looped = true
                a.playing = true
                a.play_on_awake = true
                a.play_requested = true
                a.stop_requested = false
                a.min_distance = 1.0
                a.max_distance = 50.0
                a.spatial = true

                local al = ctx:GetComponent(AudioListener)
                al.volume = 0.8
                al.active = false
                al.stereo_separation = 0.3

                local mr = ctx:GetComponent(MeshRenderer)
                mr.mesh_path = "cube.obj"
                mr.visible = false
                mr.cast_shadows = false
                mr.receive_shadows = false
                mr.color = {0.1, 0.2, 0.3, 0.4}

                local se = ctx:GetComponent(ScreenEffects)
                se.enabled = { fade = true, vignette = true, rgb_shift = false, tint = false, color_adjust = false }
                se.fade_color = {0.1, 0.2, 0.3, 0.4}
                se.vignette_strength = 0.5
                se.vignette_radius = 0.6
                se.vignette_softness = 0.7
                se.rgb_shift = {0.01, 0.02}
                se.tint_color = {0.5, 0.6, 0.7, 0.8}
                se.brightness = 1.2
                se.contrast = 0.9

                local ci = ctx:GetComponent(CursorInteractable)
                ci.is_pressed = true
                ci.is_hovered = true
                ci.was_hovered = false
                ci.bounds_size = runa.vec3(2, 3, 4)

                local odi = ctx:GetComponent(ObjectDefinitionInstance)
                odi.object_id = "enemy_01"
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
            Transform::default(),
            Collider2D::default(),
            Collider3D::default(),
            PhysicsCollision::default(),
            Sorting::default(),
            Camera::default(),
            AudioSource::default(),
            AudioListener::default(),
            MeshRenderer::default(),
            ScreenEffects::default(),
            CursorInteractable::default(),
            ObjectDefinitionInstance::default(),
            ActiveCamera::default(),
            ScriptComponent::new(path.to_str().unwrap()),
        ));

        script_system(&mut world);

        let t = world.get::<Transform>(e).unwrap();
        assert_eq!(t.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.rotation, Quat::from_xyzw(0.0, 0.0, 0.0, 1.0));
        assert_eq!(t.scale, Vec3::new(2.0, 2.0, 2.0));

        let c2 = world.get::<Collider2D>(e).unwrap();
        assert!(matches!(c2.shape, Collider2DShape::Circle { radius } if (radius - 2.5).abs() < 1e-6));
        assert_eq!(c2.offset, Vec2::new(1.0, 1.0));
        assert!(!c2.enabled);
        assert!(c2.is_trigger);
        assert_eq!(c2.layer, 7);

        let c3 = world.get::<Collider3D>(e).unwrap();
        assert!(matches!(c3.shape, Collider3DShape::Box { half_size } if half_size == Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(c3.offset, Vec3::new(0.0, 0.0, 5.0));
        assert!(!c3.enabled);
        assert!(c3.is_trigger);
        assert_eq!(c3.layer, 9);

        let pc = world.get::<PhysicsCollision>(e).unwrap();
        assert_eq!(pc.size, Vec2::new(10.0, 20.0));
        assert!(!pc.enabled);

        let s = world.get::<Sorting>(e).unwrap();
        assert_eq!(s.order, 42);
        assert!(s.y_sort);
        assert!((s.y_offset - 3.5).abs() < 1e-6);

        let cam = world.get::<Camera>(e).unwrap();
        assert_eq!(cam.position, Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(cam.target, Vec3::ZERO);
        assert_eq!(cam.up, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(cam.projection, ProjectionType::Perspective);
        assert_eq!(cam.orthographic_size, Vec2::new(40.0, 30.0));
        assert!((cam.near - 0.5).abs() < 1e-6);
        assert!((cam.far - 2000.0).abs() < 1e-6);
        assert!((cam.fov - 1.2).abs() < 1e-6);

        let a = world.get::<AudioSource>(e).unwrap();
        assert_eq!(a.source_path.as_deref(), Some("sfx.wav"));
        assert!((a.volume - 0.25).abs() < 1e-6);
        assert!(a.looped);
        assert!(a.playing);
        assert!(a.play_on_awake);
        assert!(a.play_requested);
        assert!(!a.stop_requested);
        assert!((a.min_distance - 1.0).abs() < 1e-6);
        assert!((a.max_distance - 50.0).abs() < 1e-6);
        assert!(a.spatial);

        let al = world.get::<AudioListener>(e).unwrap();
        assert!((al.volume - 0.8).abs() < 1e-6);
        assert!(!al.active);
        assert!((al.stereo_separation - 0.3).abs() < 1e-6);

        let mr = world.get::<MeshRenderer>(e).unwrap();
        assert_eq!(mr.mesh_path.as_deref(), Some("cube.obj"));
        assert!(!mr.visible);
        assert!(!mr.cast_shadows);
        assert!(!mr.receive_shadows);
        assert_eq!(mr.color, [0.1, 0.2, 0.3, 0.4]);

        let se = world.get::<ScreenEffects>(e).unwrap();
        assert!(se.enabled.fade);
        assert!(se.enabled.vignette);
        assert!(!se.enabled.rgb_shift);
        assert!(!se.enabled.tint);
        assert!(!se.enabled.color_adjust);
        assert_eq!(se.fade_color, [0.1, 0.2, 0.3, 0.4]);
        assert!((se.vignette_strength - 0.5).abs() < 1e-6);
        assert!((se.vignette_radius - 0.6).abs() < 1e-6);
        assert!((se.vignette_softness - 0.7).abs() < 1e-6);
        assert_eq!(se.rgb_shift, [0.01, 0.02]);
        assert_eq!(se.tint_color, [0.5, 0.6, 0.7, 0.8]);
        assert!((se.brightness - 1.2).abs() < 1e-6);
        assert!((se.contrast - 0.9).abs() < 1e-6);

        let ci = world.get::<CursorInteractable>(e).unwrap();
        assert!(ci.is_pressed);
        assert!(ci.is_hovered);
        assert!(!ci.was_hovered);
        assert_eq!(ci.bounds_size, Vec3::new(2.0, 3.0, 4.0));

        let odi = world.get::<ObjectDefinitionInstance>(e).unwrap();
        assert_eq!(odi.object_id, "enemy_01");

        let _ = std::fs::remove_file(&path);
    }
}
