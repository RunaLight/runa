use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::core::components::{Camera, SpriteRenderer, Transform};
use runa_engine::core::glam::Vec3;
use runa_engine::core::resources::{input::InputState, Time};
use runa_engine::core::KeyCode;
use runa_engine::ecs::W;
use runa_engine::prelude::{console_log, MessageLevel};
use runa_engine::system;
use runa_engine::{asset, ecs};

#[system]
fn player_movement(world: &mut ecs::World) {
    let speed = 8.0;
    let dt = world.get_resource::<Time>().delta;
    let mut input = world.delete_resource::<InputState>();

    for (_, transform) in world.query_mut::<W<Transform>>() {
        let mut dir = Vec3::ZERO;
        if input.is_key_pressed(KeyCode::KeyW) {
            dir.y += 1.0;
        }
        if input.is_key_pressed(KeyCode::KeyS) {
            dir.y -= 1.0;
        }
        if input.is_key_pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if input.is_key_pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        transform.position += dir.normalize_or_zero() * speed * dt;
    }

    if input.is_key_just_pressed(KeyCode::KeyF) {
        let pos = world
            .query::<ecs::R<Transform>>()
            .next()
            .map(|(_, t)| t.position);
        console_log!(world, MessageLevel::Info, "Player at {:?}", pos);
    }

    world.add_resource(input);
}

fn main() {
    let mut world = ecs::World::new();

    let texture = asset::load_image!("assets/art/Charactert.png");
    world.spawn((
        Transform {
            position: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(1., 1., 16.),
            ..Transform::default()
        },
        SpriteRenderer::new(Some(texture)),
    ));

    world.spawn((Camera::new_orthographic(32.0, 18.0),));

    let config = RunaWindowConfig {
        title: "Runa Sandbox".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
    };

    let _ = RunaApp::run_with_config(world, config);
}
