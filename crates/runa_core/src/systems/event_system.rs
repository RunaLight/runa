use std::any::{Any, TypeId};
use std::collections::HashMap;

pub trait Event: Send + 'static {}
type EventCallback = Box<dyn Fn(&dyn Any) + Send>;

/// Global event bus (singleton, like `InputState` / `AudioEngine`).
///
/// No need to spawn anything or query the world — just emit, subscribe,
/// and process from anywhere.
///
/// ```rust,ignore
/// let event_bus = world.get_resource_mut::<EventBus>();
/// event_bus.emit(MyEvent { x: 1 });
/// event_bus.subscribe(|e: &MyEvent| println!("got {}", e.x));
/// event_bus.process(); // dispatch queued events to subscribers
/// ```
#[derive(Default)]
pub struct EventBus {
    listeners: HashMap<TypeId, Vec<EventCallback>>,
    queue: Vec<Box<dyn Any + Send>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
            queue: Vec::new(),
        }
    }

    pub fn emit<E: Event>(&mut self, event: E) {
        self.queue.push(Box::new(event));
    }

    pub fn subscribe<E: Event>(&mut self, callback: impl Fn(&E) + Send + 'static) {
        let type_id = TypeId::of::<E>();
        let wrapped: EventCallback = Box::new(move |event| {
            if let Some(e) = event.downcast_ref::<E>() {
                callback(e);
            }
        });
        self.listeners.entry(type_id).or_default().push(wrapped);
    }

    pub fn process(&mut self) {
        let events = std::mem::take(&mut self.queue);
        for event in events {
            let tid = (*event).type_id();
            if let Some(callbacks) = self.listeners.get(&tid) {
                for cb in callbacks {
                    cb(event.as_ref());
                }
            }
        }
    }
}
