use std::alloc::Layout;
use std::any::TypeId;
use std::mem::needs_drop;

// ─── ComponentInfo ────────────────────────────────────────────

pub type DropFn = unsafe fn(*mut u8);

#[derive(Clone)]
pub struct ComponentInfo {
    pub type_id: TypeId,
    pub layout: Layout,
    pub drop_fn: Option<DropFn>,
}

impl ComponentInfo {
    pub fn of<T: 'static>() -> Self {
        let drop_fn = if needs_drop::<T>() {
            Some(drop_as::<T> as DropFn)
        } else {
            None
        };
        Self {
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop_fn,
        }
    }
}

unsafe fn drop_as<T>(ptr: *mut u8) {
    std::ptr::drop_in_place(ptr as *mut T)
}

// ─── BlobVec ──────────────────────────────────────────────────

pub struct BlobVec {
    info: ComponentInfo,
    data: Vec<u8>,
}

impl BlobVec {
    /// # Safety
    ///
    /// `T` must match the component type that was originally stored in this column.
    /// Misusing the type parameter leads to undefined behavior.
    pub unsafe fn new<T: 'static>() -> Self {
        Self {
            info: ComponentInfo::of::<T>(),
            data: Vec::new(),
        }
    }

    pub fn new_with_info(info: ComponentInfo) -> Self {
        Self {
            info,
            data: Vec::new(),
        }
    }

    pub fn info(&self) -> &ComponentInfo {
        &self.info
    }

    pub fn get(&self, index: usize) -> Option<*mut u8> {
        let size = self.info.layout.size();
        if index >= self.len() {
            return None;
        }
        unsafe { Some(self.data.as_ptr().add(index * size) as *mut u8) }
    }

    pub fn len(&self) -> usize {
        self.data
            .len()
            .checked_div(self.info.layout.size())
            .unwrap_or(self.data.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    /// # Safety
    ///
    /// `value` must point to a valid initialised value of the component type
    /// stored in this column. The value is copied (not moved) into storage.
    pub unsafe fn push(&mut self, value: *mut u8) {
        let size = self.info.layout.size();
        if size == 0 {
            self.data.push(0);
            return;
        }
        let offset = self.data.len();
        let new_len = offset + size;
        self.data.resize(new_len, 0);
        unsafe {
            std::ptr::copy_nonoverlapping(value, self.data.as_mut_ptr().add(offset), size);
        }
    }

    /// # Safety
    ///
    /// `index` must be in-bounds. After removal the last element is moved into
    /// `index`'s slot, so any existing reference into slot `index` is invalidated.
    pub unsafe fn swap_remove(&mut self, index: usize) {
        let size = self.info.layout.size();
        if size == 0 {
            self.data.swap_remove(index);
            return;
        }
        let len = self.data.len() / size;

        if let Some(drop) = self.info.drop_fn {
            unsafe { drop(self.data.as_mut_ptr().add(index * size)) }
        }

        if index != len - 1 {
            let src = self.data.as_ptr().add((len - 1) * size);
            let dst = self.data.as_mut_ptr().add(index * size);
            unsafe { std::ptr::copy_nonoverlapping(src, dst, size) }
        }

        unsafe { self.data.set_len((len - 1) * size) }
    }

    /// Like [`swap_remove`](Self::swap_remove) but does **not** call the component's
    /// `Drop`. Used when the element at `index` was already moved out (e.g. returned
    /// to the caller) or copied into another column, so dropping it here would be a
    /// double-free.
    ///
    /// # Safety
    ///
    /// `index` must be in-bounds and the value at `index` must no longer be owned by
    /// this column (moved out or duplicated elsewhere).
    pub unsafe fn swap_remove_no_drop(&mut self, index: usize) {
        let size = self.info.layout.size();
        if size == 0 {
            self.data.swap_remove(index);
            return;
        }
        let len = self.data.len() / size;
        if index != len - 1 {
            let src = self.data.as_ptr().add((len - 1) * size);
            let dst = self.data.as_mut_ptr().add(index * size);
            unsafe { std::ptr::copy_nonoverlapping(src, dst, size) }
        }
        unsafe { self.data.set_len((len - 1) * size) }
    }

    pub fn clear(&mut self) {
        if let Some(drop) = self.info.drop_fn {
            let size = self.info.layout.size();
            if let Some(len) = self.data.len().checked_div(size) {
                for i in 0..len {
                    unsafe { drop(self.data.as_mut_ptr().add(i * size)) }
                }
            }
        }
        self.data.clear();
    }
}
