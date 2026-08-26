use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::asset::load_image;
use runa_engine::core::components::{Camera, SpriteRenderer, Transform};
use runa_engine::core::Vec3;
use runa_engine::ecs::World;
use runa_engine::scripting::{load_script, write_luau_types};

fn main() {
    // Emit the auto-generated Luau type definitions (`scripts/runa.luau`) so
    // the editor / luau-lsp can type-check scripts. Scripts `require("runa")` to
    // get the typed, namespaced API. Re-run the app if you add a new
    // `#[derive(Scriptable)]` type elsewhere.
    let mut types_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    types_path.push("scripts/runa.luau");
    write_luau_types(&types_path);

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

    let config = RunaWindowConfig {
        title: "Lua Scripting Test".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
    };

    let _ = RunaApp::run_with_config(world, config);
}
