use std::time::Instant;

use runa_core::{components::Time, Console};
use winit::{
    error::EventLoopError,
    event_loop::{ControlFlow, EventLoop},
};

use crate::app::{App, RunaWindowConfig};

/// Default Runa App to start Application
pub struct RunaApp {}

impl RunaApp {
    fn run_with_world(
        mut world: runa_ecs::World,
        config: RunaWindowConfig,
    ) -> Result<(), EventLoopError> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);

        runa_core::input::InputState::initialize();
        let console = Console::new();

        let mut scheduler = runa_ecs::Scheduler::new();
        scheduler.collect_registered_systems("Update");

        init_resources(&mut world);

        let mut app = App {
            window: None,
            renderer: None,
            queue: runa_render_api::RenderQueue::new(),
            world,
            scheduler,
            last_time: Instant::now(),
            accumulator: 0.0,
            frame_count: 0,
            last_fps_update: Instant::now(),
            last_frame_time: 0.0,
            current_frame_time_ms: 0.0,
            current_render_time_ms: 0.0,
            current_update_time_ms: 0.0,
            interpolation_alpha: 0.0,
            frame_start: Instant::now(),
            console,
            config,
            current_fps: 0.0,
        };

        event_loop.run_app(&mut app)
    }

    pub fn run_with_config(
        world: runa_ecs::World,
        config: RunaWindowConfig,
    ) -> Result<(), EventLoopError> {
        Self::run_with_world(world, config)
    }

    pub fn run_default(world: runa_ecs::World) -> Result<(), EventLoopError> {
        Self::run_with_config(world, RunaWindowConfig::default())
    }
}

fn init_resources(world: &mut runa_ecs::World) {
    world.init_resource::<Time>();
}
