use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::core::components::{Mesh, MeshRenderer, Transform};
use runa_engine::core::glam::{Quat, Vec3};
use runa_engine::core::resources::Time;
use runa_engine::ecs::{World, R, W};
use runa_engine::system;

use crate::camera_ctrl::spawn_camera;

mod camera_ctrl;

#[system]
fn rotate_cubes(world: &mut World) {
    let dt = world.get_resource::<Time>().delta;
    for (_, (transform, _mesh)) in world.query_mut::<(W<Transform>, R<MeshRenderer>)>() {
        transform.rotation *= Quat::from_rotation_y(0.5 * dt);
    }
}

fn main() {
    let mut world = World::new();

    world.spawn((
        Transform {
            position: Vec3::new(-1.5, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            ..Transform::default()
        },
        MeshRenderer::new(Mesh::cube(1.0)),
    ));

    world.spawn((
        Transform {
            position: Vec3::new(1.5, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(-2.0, 2.0, 2.0),
            ..Transform::default()
        },
        MeshRenderer::new(Mesh::cube(1.0)),
    ));

    let _ = spawn_camera(&mut world);

    let config = RunaWindowConfig {
        title: "Runa 3D Sandbox - rotating cubes".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
    };

    let _ = RunaApp::run_with_config(world, config);
}
