use runa_engine::{
    app::{RunaApp, RunaWindowConfig},
    asset::load_image,
    core::{
        components::{Camera, SpriteRenderer, Transform},
        Vec3,
    },
    ecs::{commands, World, R},
};

struct Spawner;

fn spawn_test(world: &mut World) {
    for (_, _s) in world.query_mut::<R<Spawner>>() {
        commands().spawn((
            SpriteRenderer::new(Some(load_image!("assets/Charactert.png"))),
            Transform {
                position: Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 0.,
                },
                scale: Vec3 {
                    x: 1.,
                    y: 1.,
                    z: 16.,
                },
                ..Default::default()
            },
        ));
    }
}

fn main() {
    let mut world = World::default();

    world.spawn((Camera::new_orthographic(32.0, 32.0),));

    world.spawn((Spawner,));

    spawn_test(&mut world);

    let cfg = RunaWindowConfig {
        title: "command_queue_test".into(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
    };

    let _ = RunaApp::run_with_config(world, cfg);
}
