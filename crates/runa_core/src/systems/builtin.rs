use crate::audio::AudioEngine;
use crate::collision2d::intersects_world;
use crate::collision3d::intersects_world as intersects_world_3d;
use crate::components::{
    AudioListener, AudioSource, Collider2D, Collider3D, CursorInteractable, OnTriggerEnter2D,
    OnTriggerEnter3D, OnTriggerExit2D, OnTriggerExit3D, OnTriggerStay2D, OnTriggerStay3D,
    SpriteAnimator, SpriteRenderer, Transform, WorldCollider2D, WorldCollider3D,
};
use crate::resources::event::EventBus;
use crate::resources::input::InputState;
use crate::resources::{CollisionTracker2D, CollisionTracker3D, Time};
use runa_ecs::{Entity, R, W};
use runa_macros::system;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use winit::event::MouseButton;

fn audio_engine() -> &'static Mutex<Option<AudioEngine>> {
    static ENGINE: OnceLock<Mutex<Option<AudioEngine>>> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut e = AudioEngine::new();
        match e.initialize() {
            Ok(()) => {
                e.set_master_volume(0.5);
                Mutex::new(Some(e))
            }
            Err(err) => {
                eprintln!("audio_system: failed to initialize: {}", err);
                Mutex::new(None)
            }
        }
    })
}

#[system(Update)]
pub fn cursor_interaction(world: &mut runa_ecs::World) {
    let world_pos = match world
        .get_resource_mut::<InputState>()
        .get_mouse_world_position()
    {
        Some(p) => p,
        None => return,
    };
    let mouse_down = world
        .get_resource_mut::<InputState>()
        .is_mouse_button_just_pressed(MouseButton::Left);

    for (_, (interactable, transform)) in world.query_mut::<(W<CursorInteractable>, R<Transform>)>()
    {
        interactable.is_hovered = interactable.contains_point(world_pos, transform.position);
        if mouse_down && interactable.is_hovered {
            if let Some(cb) = interactable.on_click_mut() {
                if let Ok(f) = cb.get_mut() {
                    f();
                }
            }
        }
        interactable.update_callbacks();
    }
}

#[system(Update)]
pub fn audio_system(world: &mut runa_ecs::World) {
    let mut guard = audio_engine().lock().unwrap();
    let Some(engine) = guard.as_mut() else {
        return;
    };

    for (_, source) in world.query_mut::<W<AudioSource>>() {
        if source.play_requested {
            source.sound_id = engine.play(source);
            source.play_requested = false;
            source.playing = source.sound_id.is_some();
        }
        if source.stop_requested {
            if let Some(id) = source.sound_id {
                engine.stop(id);
            }
            source.sound_id = None;
            source.stop_requested = false;
            source.playing = false;
        }
    }

    for (_, (listener, transform)) in world.query::<(R<AudioListener>, R<Transform>)>() {
        if listener.active {
            engine.set_listener(transform.position, transform.rotation, listener.volume);
        }
    }

    engine.update_spatial_volumes();
    engine.cleanup();
}

#[system(Update)]
pub fn eventbus_system(world: &mut runa_ecs::World) {
    world.get_resource_mut::<EventBus>().process();
}

#[system(Update)]
pub fn sprite_animator_system(world: &mut runa_ecs::World) {
    let dt = world.get_resource::<Time>().delta;
    for (_, (animator, sprite)) in world.query_mut::<(W<SpriteAnimator>, W<SpriteRenderer>)>() {
        let uv = animator.tick(dt);
        sprite.uv_rect = uv;
    }
}

struct Collider2DSnapshot {
    entity: Entity,
    collider: Collider2D,
    world: WorldCollider2D,
}

#[system(Update)]
pub fn collision_2d_system(world: &mut runa_ecs::World) {
    // ── Pass 1: read. Copy each enabled collider into a Vec, resolved to
    //    world space exactly once. The world borrow ends here. ──────────────
    let mut colliders: Vec<Collider2DSnapshot> = Vec::new();

    for (entity, (t, c)) in world.query::<(R<Transform>, R<Collider2D>)>() {
        let collider = *c;
        if !collider.enabled {
            continue;
        }
        colliders.push(Collider2DSnapshot {
            entity,
            collider,
            world: collider.to_world(t),
        });
    }

    // ── Pass 2: narrow. Pure math on the Vec, no world access. ─────────────
    let n = colliders.len();
    let mut current_contacts: HashMap<Entity, HashSet<Entity>> = HashMap::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if intersects_world(&colliders[i].world, &colliders[j].world) {
                let a = colliders[i].entity;
                let b = colliders[j].entity;
                current_contacts.entry(a).or_default().insert(b);
                current_contacts.entry(b).or_default().insert(a);
            }
        }
    }

    // ── Pass 3: write. Diff against last frame, emit trigger events. ────────
    world.init_resource::<CollisionTracker2D>();
    let is_trigger: HashMap<Entity, bool> = colliders
        .iter()
        .map(|c| (c.entity, c.collider.is_trigger))
        .collect();

    let events: Vec<(Entity, Entity, u8)> = {
        let tracker = world.get_resource_mut::<CollisionTracker2D>();
        let prev = &tracker.contacts;
        let mut out = Vec::new();
        for (entity, others) in &current_contacts {
            let prev_others = prev.get(entity);
            for other in others {
                let was = prev_others.is_some_and(|s| s.contains(other));
                out.push((*entity, *other, if was { 2 } else { 0 }));
            }
            if let Some(prev_others) = prev_others {
                for other in prev_others {
                    if !others.contains(other) {
                        out.push((*entity, *other, 1));
                    }
                }
            }
        }
        tracker.contacts = current_contacts;
        out
    };

    let bus = world.get_resource_mut::<EventBus>();
    for (this, other, kind) in events {
        let trigger = is_trigger.get(&this).copied().unwrap_or(false)
            || is_trigger.get(&other).copied().unwrap_or(false);
        if !trigger {
            continue;
        }
        match kind {
            0 => bus.emit(OnTriggerEnter2D { this, other }),
            1 => bus.emit(OnTriggerExit2D { this, other }),
            _ => bus.emit(OnTriggerStay2D { this, other }),
        }
    }
}

struct Collider3DSnapshot {
    entity: Entity,
    collider: Collider3D,
    world: WorldCollider3D,
}

#[system(Update)]
pub fn collision_3d_system(world: &mut runa_ecs::World) {
    let mut colliders: Vec<Collider3DSnapshot> = Vec::new();

    for (entity, (t, c)) in world.query::<(R<Transform>, R<Collider3D>)>() {
        let collider = *c;
        if !collider.enabled {
            continue;
        }
        colliders.push(Collider3DSnapshot {
            entity,
            collider,
            world: collider.to_world(t),
        });
    }

    let n = colliders.len();
    let mut current_contacts: HashMap<Entity, HashSet<Entity>> = HashMap::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if intersects_world_3d(&colliders[i].world, &colliders[j].world) {
                let a = colliders[i].entity;
                let b = colliders[j].entity;
                current_contacts.entry(a).or_default().insert(b);
                current_contacts.entry(b).or_default().insert(a);
            }
        }
    }

    world.init_resource::<CollisionTracker3D>();
    let is_trigger: HashMap<Entity, bool> = colliders
        .iter()
        .map(|c| (c.entity, c.collider.is_trigger))
        .collect();

    let events: Vec<(Entity, Entity, u8)> = {
        let tracker = world.get_resource_mut::<CollisionTracker3D>();
        let prev = &tracker.contacts;
        let mut out = Vec::new();
        for (entity, others) in &current_contacts {
            let prev_others = prev.get(entity);
            for other in others {
                let was = prev_others.is_some_and(|s| s.contains(other));
                out.push((*entity, *other, if was { 2 } else { 0 }));
            }
            if let Some(prev_others) = prev_others {
                for other in prev_others {
                    if !others.contains(other) {
                        out.push((*entity, *other, 1));
                    }
                }
            }
        }
        tracker.contacts = current_contacts;
        out
    };

    let bus = world.get_resource_mut::<EventBus>();
    for (this, other, kind) in events {
        let trigger = is_trigger.get(&this).copied().unwrap_or(false)
            || is_trigger.get(&other).copied().unwrap_or(false);
        if !trigger {
            continue;
        }
        match kind {
            0 => bus.emit(OnTriggerEnter3D { this, other }),
            1 => bus.emit(OnTriggerExit3D { this, other }),
            _ => bus.emit(OnTriggerStay3D { this, other }),
        }
    }
}
