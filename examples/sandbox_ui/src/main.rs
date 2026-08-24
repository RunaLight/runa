use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::core::components::{Camera, Transform, UiRenderer};
use runa_engine::core::ecs::{World, W};
use runa_engine::core::glam::Vec3;
use runa_engine::core::resources::Time;
use runa_engine::core::ui::{CanvasSpace, TextHandle};
use runa_engine::system;

/// Component (sits on the same entity as the screen `UiRenderer`) holding the
/// handle of the text node whose color is animated every frame.
#[derive(Clone, Copy)]
struct PulseText {
    handle: TextHandle,
}

fn ui_builder(ui: &mut UiRenderer) -> Option<TextHandle> {
    ui.clear();

    if matches!(ui.space, CanvasSpace::Screen) {
        let mut pulse_handle: Option<TextHandle> = None;

        ui.vbox(|ui| {
            ui.add_text("Runa Engine UI Demo")
                .with_font_size(28)
                .with_text_color(0.0, 0.8, 1.0, 1.0);

            // Text whose color is animated at runtime via a typed handle.
            pulse_handle = Some(
                ui.add_text("RGB Pulse")
                    .with_font_size(22)
                    .with_text_color(1.0, 1.0, 1.0, 1.0)
                    .into_text(),
            );

            let s: String = "RichText".into();

            ui.add_text(format!("This is a <b>{s}</b> example."))
                .with_font_size(16)
                .with_text_color(0.8, 0.8, 0.8, 1.0);

            ui.add_button(
                Some("Click Me"),
                Some(Box::new(|| {
                    println!("Button clicked!");
                })),
            )
            .with_background(0.2, 0.5, 0.8, 1.0)
            .with_size(160.0, 40.0);

            ui.add_slider()
                .with_slider_value(0.5)
                .with_slider_range(0.0, 1.0)
                .with_size(200.0, 24.0)
                .id();
        })
        .with_background(0.1, 0.1, 0.15, 0.9)
        .with_padding(16.0, 16.0, 16.0, 16.0)
        .with_gap(8.0)
        .with_pos(40.0, 40.0)
        .with_size(300.0, 400.0);

        pulse_handle
    } else {
        ui.vbox(|ui| {
            ui.add_text("World-Space UI")
                .with_font_size(24)
                .with_text_color(1.0, 0.8, 0.0, 1.0);

            ui.add_text("Attached to entity Transform at (170, 0).\nLocal offset (0, 0) — panel follows entity.")
                .with_font_size(13)
                .with_text_color(0.9, 0.9, 0.9, 1.0);

            ui.add_button(Some("World Button"), Some(Box::new(|| {
                println!("World button clicked!");
            })))
            .with_background(0.6, 0.3, 0.1, 1.0)
            .with_size(140.0, 36.0);
        })
        .with_background(0.15, 0.1, 0.05, 0.9)
        .with_padding(12.0, 12.0, 12.0, 12.0)
        .with_gap(6.0)
        .with_pos(0.0, 0.0)
        .with_size(160.0, 200.0);

        None
    }
}

/// Animates the `PulseText` node's color through the RGB wheel each frame.
#[system]
fn pulse_text_system(world: &mut World) {
    let t = world.get_resource::<Time>().elapsed;

    let r = (t * 2.0).sin() * 0.5 + 0.5;
    let g = ((t * 2.0) + 2.094).sin() * 0.5 + 0.5;
    let b = ((t * 2.0) + 4.188).sin() * 0.5 + 0.5;

    for (_, (ui, pulse)) in world.query_mut::<(W<UiRenderer>, W<PulseText>)>() {
        pulse.handle.set_color(ui, [r, g, b, 1.0]);
    }
}

fn main() {
    let mut world = World::new();

    world.spawn((Camera::new_orthographic(320.0, 180.0),));

    let mut ui = UiRenderer::new(CanvasSpace::Screen);
    let pulse = ui_builder(&mut ui);

    if let Some(handle) = pulse {
        world.spawn((ui, PulseText { handle }));
    } else {
        world.spawn((ui,));
    }

    let mut ui_w = UiRenderer::new(CanvasSpace::World);
    ui_builder(&mut ui_w);

    world.spawn((
        ui_w,
        Transform {
            position: Vec3::new(170.0, 0.0, 0.0),
            scale: Vec3::ONE,
            ..Default::default()
        },
    ));

    let config = RunaWindowConfig {
        title: "Runa UI Demo — Screen (left) + World (right, entity-attached)".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
    };

    let _ = RunaApp::run_with_config(world, config);
}
