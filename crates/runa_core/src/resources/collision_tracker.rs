use std::collections::{HashMap, HashSet};

use runa_ecs::Entity;

/// System-owned resource that remembers each entity's contacts from the
/// previous frame, so enter/exit/stay can be diffed.
#[derive(Default)]
pub struct CollisionTracker2D {
    pub contacts: HashMap<Entity, HashSet<Entity>>,
}

#[derive(Default)]
pub struct CollisionTracker3D {
    pub contacts: HashMap<Entity, HashSet<Entity>>,
}
