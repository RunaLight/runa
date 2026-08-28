use std::sync::Once;

use runa_engine::{
    app::{RunaApp, RunaWindowConfig},
    core::{components::Camera, console_log, MessageLevel},
    ecs::{self, World},
    system,
};

fn main() {
    let mut world = ecs::World::new();

    world.spawn((Camera::new_orthographic(32.0, 18.0),));

    let config = RunaWindowConfig {
        title: "Runa Sandbox".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
        luau_types_path: None,
    };

    let _ = RunaApp::run_with_config(world, config);
}

static INIT: Once = Once::new();

#[system]
fn test(world: &mut World) {
    INIT.call_once(|| {
        console_log!(world, "Default message test");
        console_log!(world, MessageLevel::Info, "Info message test");
        console_log!(world, MessageLevel::Warning, "Warning message test");
        console_log!(world, MessageLevel::Error, "Error message test");
    });
}
