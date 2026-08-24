use runa_engine::core::{
    components::{Camera, Transform},
    ecs::{World, W},
    glam::{Quat, Vec3},
    resources::{
        input::{lock_cursor, show_cursor, InputState},
        Time,
    },
    KeyCode,
};
use runa_engine::system;

struct CameraController {
    yaw: f32,
    pitch: f32,
    sensitivity: f32,
    speed: f32,
}

impl CameraController {
    fn new(sensitivity: f32, speed: f32) -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            sensitivity,
            speed,
        }
    }
}

#[system]
fn camera_controller_system(world: &mut World) {
    show_cursor(false);
    lock_cursor(true);
    let dt = world.get_resource::<Time>().delta;

    let mut input = world.delete_resource::<InputState>();

    let mouse = input.mouse_delta;
    let (dx, dy) = (mouse.0, mouse.1);

    for (_, (transform, ctrl)) in world.query_mut::<(W<Transform>, W<CameraController>)>() {
        ctrl.yaw -= dx * ctrl.sensitivity;
        ctrl.pitch -= dy * ctrl.sensitivity;
        ctrl.pitch = ctrl.pitch.clamp(-89.0, 89.0);
        transform.rotation = Quat::from_rotation_y(ctrl.yaw.to_radians())
            * Quat::from_rotation_x(ctrl.pitch.to_radians());

        let forward = transform.rotation * -Vec3::Z;
        let right = transform.rotation * Vec3::X;
        let mut move_dir = Vec3::ZERO;
        if input.is_key_pressed(KeyCode::KeyW) {
            move_dir += forward;
        }
        if input.is_key_pressed(KeyCode::KeyS) {
            move_dir -= forward;
        }
        if input.is_key_pressed(KeyCode::KeyD) {
            move_dir += right;
        }
        if input.is_key_pressed(KeyCode::KeyA) {
            move_dir -= right;
        }
        if input.is_key_pressed(KeyCode::Space) {
            move_dir += Vec3::Y;
        }
        if input.is_key_pressed(KeyCode::ShiftLeft) {
            move_dir -= Vec3::Y;
        }

        transform.position += move_dir.normalize_or_zero() * ctrl.speed * dt;
    }

    world.add_resource(input);
}

pub fn spawn_camera(world: &mut World) -> u64 {
    world.spawn((
        Transform {
            position: Vec3 {
                x: 0.,
                y: 0.,
                z: 10.,
            },
            ..Default::default()
        },
        CameraController::new(0.1, 5.0),
        Camera::new_perspective(Vec3::default(), Vec3::NEG_Z, Vec3::Y, 90.0, 0.1, 1000.0),
    ))
}
