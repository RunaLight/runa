use crate::{commands::apply_commands, World};

pub trait System: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn run(&mut self, world: &mut World);
}

pub struct FunctionSystem {
    name: &'static str,
    func: fn(&mut World),
}

impl FunctionSystem {
    pub fn new(name: &'static str, func: fn(&mut World)) -> Self {
        Self { name, func }
    }
}

impl System for FunctionSystem {
    fn name(&self) -> &'static str {
        self.name
    }
    fn run(&mut self, world: &mut World) {
        (self.func)(world)
    }
}

// ─── Stage ──────────────────────────────────────────────────

/// Execution stage a system belongs to.
///
/// `Start` systems run once at startup (after resources are initialized);
/// `Update` systems run every frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Start,
    Update,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Start => "Start",
            Stage::Update => "Update",
        }
    }
}

// ─── Auto-registration via inventory ─────────────────────────

pub struct SystemDescriptor {
    pub name: &'static str,
    pub func: fn(&mut World),
    pub stage: Stage,
}

inventory::collect!(SystemDescriptor);

// ─── SystemStage ─────────────────────────────────────────────

pub struct SystemStage {
    pub name: &'static str,
    pub systems: Vec<Box<dyn System>>,
}

impl SystemStage {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            systems: Vec::new(),
        }
    }

    pub fn add_system(&mut self, system: impl System) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn run(&mut self, world: &mut World) {
        for sys in &mut self.systems {
            sys.run(world);
        }
    }
}

// ─── Scheduler ───────────────────────────────────────────────

pub struct Scheduler {
    pub stages: Vec<SystemStage>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: SystemStage) -> &mut Self {
        self.stages.push(stage);
        self
    }

    pub fn collect_registered_systems(&mut self) -> &mut Self {
        use std::collections::HashMap;

        let mut by_stage: HashMap<Stage, SystemStage> = HashMap::new();
        for desc in inventory::iter::<SystemDescriptor> {
            let stage = by_stage
                .entry(desc.stage)
                .or_insert_with(|| SystemStage::new(desc.stage.name()));
            stage.add_system(FunctionSystem::new(desc.name, desc.func));
        }

        // Deterministic order: Start, then Update, then any future stages.
        let mut ordered: Vec<SystemStage> = Vec::new();
        for s in [Stage::Start, Stage::Update] {
            if let Some(stage) = by_stage.remove(&s) {
                ordered.push(stage);
            }
        }
        for (_, stage) in by_stage {
            ordered.push(stage);
        }
        for stage in ordered {
            self.add_stage(stage);
        }
        self
    }

    /// Run only the systems belonging to `stage` (and flush deferred commands).
    pub fn run_stage(&mut self, stage: Stage, world: &mut World) {
        for s in &mut self.stages {
            if s.name == stage.name() {
                s.run(world);
                apply_commands(world);
                return;
            }
        }
    }

    pub fn run(&mut self, world: &mut World) {
        for stage in &mut self.stages {
            stage.run(world);
        }
        apply_commands(world);
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
