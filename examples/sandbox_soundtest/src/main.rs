use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::core::components::{AudioListener, AudioSource, Camera, Transform};
use runa_engine::core::resources::input::InputState;
use runa_engine::core::KeyCode;
use runa_engine::system;
use runa_engine::{asset, ecs};

#[system]
fn toggle_sound(world: &mut ecs::World) {
    let mut input = world.delete_resource::<InputState>();
    if input.is_key_just_pressed(KeyCode::Space) {
        for (_, source) in world.query_mut::<ecs::W<AudioSource>>() {
            if source.playing {
                source.stop();
            } else {
                source.play();
            }
        }
    }
    world.add_resource(input);
}

fn main() {
    let mut world = ecs::World::new();

    world.spawn((Camera::new_orthographic(320.0, 180.0),));
    world.spawn((AudioListener::new(), Transform::default()));

    let audio_asset = asset::load_audio!("assets/audio/test.ogg");
    let mut source = AudioSource::with_asset(audio_asset);
    source.looped = true;
    source.play();

    world.spawn((source,));

    let config = RunaWindowConfig {
        title: "Runa Sound Test — Space to toggle".to_string(),
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
