use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::core::components::{Camera, SpriteRenderer, Time, Transform};
use runa_engine::core::glam::Vec3;
use runa_engine::core::input::InputState;
use runa_engine::core::KeyCode;
use runa_engine::system;
use runa_engine::{asset, ecs};

#[system]
fn player_movement(world: &mut ecs::World) {
    let speed = 8.0;
    let dt = world.get_resource::<Time>().unwrap().delta;

    for (_, transform) in world.query_mut::<ecs::W<Transform>>() {
        let mut dir = Vec3::ZERO;
        if InputState::is_key_pressed(KeyCode::KeyW) {
            dir.y += 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyS) {
            dir.y -= 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        transform.position += dir.normalize_or_zero() * speed * dt;
    }
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
