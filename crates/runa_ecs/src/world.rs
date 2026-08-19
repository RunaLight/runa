use std::any::{self, Any, TypeId};
use std::collections::HashMap;

use crate::archetype::{Archetype, ArchetypeId, BlobColumn, Bundle};
use crate::blob_vec::ComponentInfo;
use crate::Entity;

#[derive(Clone, Copy)]
pub(crate) struct Location {
    pub archetype_id: ArchetypeId,
    pub row: u32,
}

pub struct World {
    pub archetypes: Vec<Archetype>,
    resources: HashMap<TypeId, Box<dyn Any>>,
    archetype_by_key: HashMap<Vec<TypeId>, ArchetypeId>,
    next_archetype_id: u32,
    entity_location: HashMap<Entity, Location>,
    next_entity: u64,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            resources: HashMap::new(),
            archetype_by_key: HashMap::new(),
            next_archetype_id: 0,
            entity_location: HashMap::new(),
            next_entity: 1,
        }
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let type_ids = B::type_ids();
        let infos = B::component_infos();
        let key = type_ids.clone();

        let arch_id = self.find_or_create_archetype(&key, &infos);
        let arch = &mut self.archetypes[arch_id.0 as usize];

        let entity = self.next_entity;
        self.next_entity += 1;
        let row = arch.entity_count();

        arch.entities.push(entity);
        bundle.put(&mut arch.columns);
        self.entity_location.insert(
            entity,
            Location {
                archetype_id: arch_id,
                row: row as u32,
            },
        );

        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        let Some(loc) = self.entity_location.remove(&entity) else {
            return false;
        };
        let arch = &mut self.archetypes[loc.archetype_id.0 as usize];
        let row = loc.row as usize;

        for col in &mut arch.columns {
            unsafe { col.blob.swap_remove(row) }
        }
        let last = arch.entities.swap_remove(row);

        if row < arch.entities.len() {
            if let Some(last_loc) = self.entity_location.get_mut(&last) {
                last_loc.row = row as u32;
            }
        }

        true
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let loc = self.entity_location.get(&entity)?;
        let arch = self.archetypes.get(loc.archetype_id.0 as usize)?;
        let col = arch.column(TypeId::of::<T>())?;
        let ptr = col.blob.get(loc.row as usize)? as *const T;
        unsafe { Some(&*ptr) }
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let loc = self.entity_location.get(&entity)?;
        let arch = self.archetypes.get_mut(loc.archetype_id.0 as usize)?;
        let col = arch.column_mut(TypeId::of::<T>())?;
        let ptr = col.blob.get(loc.row as usize)? as *mut T;
        unsafe { Some(&mut *ptr) }
    }

    pub fn clear(&mut self) {
        self.archetypes.clear();
        self.resources.clear();
        self.archetype_by_key.clear();
        self.next_archetype_id = 0;
        self.entity_location.clear();
        self.next_entity = 1;
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entity_location.contains_key(&entity)
    }

    pub fn entity_count(&self) -> usize {
        self.entity_location.len()
    }

    fn find_or_create_archetype(&mut self, key: &[TypeId], infos: &[ComponentInfo]) -> ArchetypeId {
        if let Some(&id) = self.archetype_by_key.get(key) {
            return id;
        }
        let id = ArchetypeId(self.next_archetype_id);
        self.next_archetype_id += 1;

        let columns: Vec<BlobColumn> = infos
            .iter()
            .map(|info| BlobColumn::new(info.clone()))
            .collect();
        let arch = Archetype::new(id, columns);
        self.archetypes.push(arch);
        self.archetype_by_key.insert(key.to_vec(), id);
        id
    }

    /// Adds a resource of type `T` to the world.
    ///
    /// # Panics
    /// Panics if a resource of type `T` is already present in the world.
    pub fn add_resource<T: 'static>(&mut self, resource: T) {
        let key = TypeId::of::<T>();

        if self.resources.contains_key(&key) {
            panic!("Resource {} already added.", any::type_name::<T>());
        }

        self.resources.insert(key, Box::new(resource));
    }

    pub fn delete_resource<T: 'static>(&mut self) -> Option<T> {
        let key = TypeId::of::<T>();
        self.resources
            .remove(&key)
            .and_then(|b| b.downcast::<T>().ok())
            .map(|b| *b)
    }

    pub fn get_resource<T: 'static>(&self) -> &T {
        let key = TypeId::of::<T>();
        self.resources
            .get(&key)
            .and_then(|b| b.downcast_ref::<T>())
            .expect(&format!("Resource {} not found.", any::type_name::<T>()))
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> &mut T {
        let key = TypeId::of::<T>();
        self.resources
            .get_mut(&key)
            .and_then(|b| b.downcast_mut::<T>())
            .expect(&format!("Resource {} not found.", any::type_name::<T>()))
    }

    /// Inserts the default value of `T` into the resource store
    /// if a resource of type `T` is not already present.
    pub fn init_resource<T: Default + 'static>(&mut self) {
        self.resources
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(T::default()));
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
