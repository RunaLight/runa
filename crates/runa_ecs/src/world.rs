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

    /// Adds (or, if already present, replaces) a component on an existing entity.
    ///
    /// This moves the entity into a new archetype (`old types + T`). Returns `false`
    /// if the entity does not exist.
    pub fn add_component<T: 'static>(&mut self, entity: Entity, value: T) -> bool {
        // Upsert: replace in place when the component is already attached.
        if let Some(slot) = self.get_mut::<T>(entity) {
            *slot = value;
            return true;
        }

        let loc = match self.entity_location.get(&entity) {
            Some(l) => *l,
            None => return false,
        };
        let old_arch_id = loc.archetype_id;
        let old_row = loc.row as usize;

        let old_infos: Vec<ComponentInfo> = self.archetypes[old_arch_id.0 as usize]
            .columns
            .iter()
            .map(|c| c.info.clone())
            .collect();
        let mut new_infos = old_infos.clone();
        new_infos.push(ComponentInfo::of::<T>());
        let new_key: Vec<TypeId> = new_infos.iter().map(|i| i.type_id).collect();
        let new_arch_id = self.find_or_create_archetype(&new_key, &new_infos);

        let new_row;
        {
            let (old_arch, new_arch) =
                Self::split_two(&mut self.archetypes, old_arch_id.0, new_arch_id.0);
            new_row = new_arch.entities.len();
            for (i, col) in old_arch.columns.iter().enumerate() {
                if let Some(src) = col.blob.get(old_row) {
                    // Copy the component's bytes into the new column; the old column
                    // is later removed with `swap_remove_no_drop`, so ownership is
                    // transferred exactly once (no double-free).
                    unsafe { new_arch.columns[i].blob.push(src) };
                }
            }
            let val_ptr = &value as *const T as *mut u8;
            unsafe { new_arch.columns[old_infos.len()].blob.push(val_ptr) };
            std::mem::forget(value);
            new_arch.entities.push(entity);
        }
        self.entity_location.insert(
            entity,
            Location {
                archetype_id: new_arch_id,
                row: new_row as u32,
            },
        );

        unsafe { self.remove_from_archetype_no_drop(old_arch_id, old_row) };
        true
    }

    /// Removes component `T` from an entity, returning it. Returns `None` if the
    /// entity or the component is absent. Moves the entity into a new archetype
    /// (`old types - T`).
    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let loc = self.entity_location.get(&entity)?;
        let old_arch_id = loc.archetype_id;
        let old_row = loc.row as usize;
        let old_arch = &self.archetypes[old_arch_id.0 as usize];
        if !old_arch.has_type(TypeId::of::<T>()) {
            return None;
        }

        // Move the `T` value out (it is returned to the caller; not dropped here).
        let t_col = old_arch.column(TypeId::of::<T>()).unwrap();
        let t_src = t_col.blob.get(old_row).unwrap() as *const T;
        let value = unsafe { std::ptr::read(t_src) };

        let old_infos: Vec<ComponentInfo> =
            old_arch.columns.iter().map(|c| c.info.clone()).collect();
        let new_infos: Vec<ComponentInfo> = old_infos
            .iter()
            .filter(|i| i.type_id != TypeId::of::<T>())
            .cloned()
            .collect();
        let new_key: Vec<TypeId> = new_infos.iter().map(|i| i.type_id).collect();
        let new_arch_id = self.find_or_create_archetype(&new_key, &new_infos);

        let new_row;
        {
            let (old_arch, new_arch) =
                Self::split_two(&mut self.archetypes, old_arch_id.0, new_arch_id.0);
            new_row = new_arch.entities.len();
            for col in &old_arch.columns {
                if col.type_id() == TypeId::of::<T>() {
                    continue;
                }
                let idx = new_arch
                    .columns
                    .iter()
                    .position(|c| c.type_id() == col.type_id())
                    .unwrap();
                if let Some(src) = col.blob.get(old_row) {
                    unsafe { new_arch.columns[idx].blob.push(src) };
                }
            }
            new_arch.entities.push(entity);
        }
        self.entity_location.insert(
            entity,
            Location {
                archetype_id: new_arch_id,
                row: new_row as u32,
            },
        );

        // Remove the entity from the old archetype. The `T` column is already moved
        // out (no drop); the rest were copied, so also use `swap_remove_no_drop`.
        let arch = &mut self.archetypes[old_arch_id.0 as usize];
        let last = arch.entities.len() - 1;
        let swapped = if old_row != last {
            Some(arch.entities[last])
        } else {
            None
        };
        for col in &mut arch.columns {
            unsafe { col.blob.swap_remove_no_drop(old_row) };
        }
        arch.entities.swap_remove(old_row);
        if let Some(e) = swapped {
            if let Some(l) = self.entity_location.get_mut(&e) {
                l.row = old_row as u32;
            }
        }

        Some(value)
    }

    /// Removes the entity's row from `arch_id` without dropping any component (the
    /// components were copied into a new archetype). Fixes the location of the
    /// entity that gets swapped into the freed slot.
    unsafe fn remove_from_archetype_no_drop(&mut self, arch_id: ArchetypeId, row: usize) {
        let arch = &mut self.archetypes[arch_id.0 as usize];
        let last = arch.entities.len() - 1;
        let swapped = if row != last {
            Some(arch.entities[last])
        } else {
            None
        };
        for col in &mut arch.columns {
            col.blob.swap_remove_no_drop(row);
        }
        arch.entities.swap_remove(row);
        if let Some(e) = swapped {
            if let Some(l) = self.entity_location.get_mut(&e) {
                l.row = row as u32;
            }
        }
    }

    /// Returns mutable borrows of two distinct archetypes so their columns can be
    /// copied between without the borrow checker complaining about a single
    /// `&mut self.archetypes` borrow.
    fn split_two(archetypes: &mut [Archetype], a: u32, b: u32) -> (&mut Archetype, &mut Archetype) {
        if a < b {
            let (front, back) = archetypes.split_at_mut(b as usize);
            (&mut front[a as usize], &mut back[0])
        } else {
            let (front, back) = archetypes.split_at_mut(a as usize);
            (&mut back[0], &mut front[b as usize])
        }
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

    pub fn delete_resource<T: 'static>(&mut self) -> T {
        let key = TypeId::of::<T>();
        self.resources
            .remove(&key)
            .and_then(|b| b.downcast::<T>().ok())
            .map(|b| *b)
            .unwrap_or_else(|| panic!("Resource {} not found.", any::type_name::<T>()))
    }

    pub fn get_resource<T: 'static>(&self) -> &T {
        let key = TypeId::of::<T>();
        self.resources
            .get(&key)
            .and_then(|b| b.downcast_ref::<T>())
            .unwrap_or_else(|| panic!("Resource {} not found.", any::type_name::<T>()))
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> &mut T {
        let key = TypeId::of::<T>();
        self.resources
            .get_mut(&key)
            .and_then(|b| b.downcast_mut::<T>())
            .unwrap_or_else(|| panic!("Resource {} not found.", any::type_name::<T>()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Default)]
    struct A {
        v: i32,
    }
    #[derive(Debug, PartialEq)]
    struct B {
        s: String,
    }

    #[test]
    fn add_and_remove_component() {
        let mut w = World::new();
        let e = w.spawn((A { v: 1 },));
        assert!(w.get::<A>(e).is_some());
        assert!(w.get::<B>(e).is_none());

        // Add a new component.
        assert!(w.add_component(e, B { s: "hi".into() }));
        assert_eq!(w.get::<B>(e).unwrap().s, "hi");
        // Existing component is untouched.
        assert_eq!(w.get::<A>(e).unwrap().v, 1);

        // Upsert replaces in place.
        assert!(w.add_component(e, B { s: "bye".into() }));
        assert_eq!(w.get::<B>(e).unwrap().s, "bye");

        // Remove returns the value and detaches it.
        let removed = w.remove_component::<B>(e).unwrap();
        assert_eq!(removed.s, "bye");
        assert!(w.get::<B>(e).is_none());
        assert!(w.get::<A>(e).is_some());

        // Removing an absent component yields None.
        assert!(w.remove_component::<B>(e).is_none());
        // Removing from a nonexistent entity yields None.
        assert!(w.remove_component::<A>(999).is_none());
    }

    #[test]
    fn add_component_nonexistent_entity() {
        let mut w = World::new();
        assert!(!w.add_component(123, A { v: 1 }));
    }
}
