use runa_engine::app::{RunaApp, RunaWindowConfig};
use runa_engine::core::components::UiRenderer;
use runa_engine::core::components::{Camera, Transform};
use runa_engine::core::glam::Vec3;
use runa_engine::core::ui::CanvasSpace;
use runa_engine::ecs;

fn ui_builder(ui: &mut UiRenderer) {
    ui.clear();

    if matches!(ui.space, CanvasSpace::Screen) {
        ui.vbox(|ui| {
            ui.add_text("Runa Engine UI Demo")
                .with_font_size(28)
                .with_text_color(0.0, 0.8, 1.0, 1.0);

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
    }

    if matches!(ui.space, CanvasSpace::World) {
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
    }
}

fn main() {
    let mut world = ecs::World::new();

    world.spawn((Camera::new_orthographic(320.0, 180.0),));

    let mut ui = UiRenderer::new(CanvasSpace::Screen);
    ui_builder(&mut ui);
    world.spawn((ui,));

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
