use crate::{Bundle, Entity, World};
use std::cell::RefCell;

type CommandFn = Box<dyn FnOnce(&mut World)>;

thread_local! {
    static QUEUE: RefCell<Vec<CommandFn>> = RefCell::new(Vec::new());
}

pub struct CommandQueue;

impl CommandQueue {
    pub fn spawn(&self, bundle: impl Bundle + 'static) {
        let closure = move |world: &mut World| {
            world.spawn(bundle);
        };
        QUEUE.with(|cell| cell.borrow_mut().push(Box::new(closure)));
    }

    pub fn despawn(&self, entity: Entity) {
        let closure = move |world: &mut World| {
            world.despawn(entity);
        };
        QUEUE.with(|cell| cell.borrow_mut().push(Box::new(closure)));
    }

    pub fn clear(&self) {
        let closure = move |world: &mut World| {
            world.clear();
        };
        QUEUE.with(|cell| cell.borrow_mut().push(Box::new(closure)));
    }
}

pub fn commands() -> CommandQueue {
    CommandQueue
}

pub fn apply_commands(world: &mut World) {
    let to_apply = QUEUE.with(|cell| cell.borrow_mut().drain(..).collect::<Vec<_>>());
    for cmd in to_apply {
        cmd(world);
    }
}
