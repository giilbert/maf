use std::{ops::Index, ptr::NonNull};

/// Vector with generational indices
#[derive(Debug, Clone)]
pub struct GenVec<T> {
    storage: Vec<GenEntry<T>>,
    free_list: Vec<u32>,
}

#[derive(Debug, Clone)]
struct GenEntry<T> {
    generation: u32,
    item: Option<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenIdx {
    generation: u32,
    index: u32,
}

impl<T> GenVec<T> {
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            free_list: Vec::new(),
        }
    }

    /// Replaces the item at the given raw index with a new item, returning the new [`GenIdx`] or
    /// `None` if the index is invalid.
    ///
    /// # Panics
    ///
    /// Panic if the index is out of bounds
    pub fn replace_index(&mut self, idx: u32, item: T) -> GenIdx {
        self.storage
            .get_mut(idx as usize)
            .map(|old_entry| {
                old_entry.generation += 1;
                old_entry.item = Some(item);

                GenIdx {
                    generation: old_entry.generation,
                    index: idx,
                }
            })
            .expect("index out of bounds")
    }

    /// Pushes an item to the end of the vector, without reusing a free index.
    pub fn push_end(&mut self, item: T) -> GenIdx {
        let index = self.storage.len() as u32;
        self.storage.push(GenEntry {
            generation: 0,
            item: Some(item),
        });

        GenIdx {
            generation: 0,
            index,
        }
    }

    /// Pushes an item into the vector, possibly reusing a free index.
    ///
    /// - If the vector is empty, it will push the item to the end.
    /// - If the vector is not empty, it will reuse a free index if available. If no free index is
    ///   available, it will push the item to the end.
    pub fn push(&mut self, item: T) -> GenIdx {
        match self.free_list.pop() {
            Some(index) => self.replace_index(index, item),
            None => self.push_end(item),
        }
    }

    /// Borrows the item at the given index, or `None` if the index is invalid.
    pub fn get(&self, idx: GenIdx) -> Option<&T> {
        let entry = self.storage.get(idx.index as usize)?;
        if entry.generation == idx.generation {
            entry.item.as_ref()
        } else {
            None
        }
    }

    /// Mutably borrows the item at the given index, or `None` if the index is invalid.
    pub fn get_mut(&mut self, idx: GenIdx) -> Option<&mut T> {
        let entry = self.storage.get_mut(idx.index as usize)?;
        if entry.generation == idx.generation {
            entry.item.as_mut()
        } else {
            None
        }
    }

    /// Returns the item at the given index, or `None` if the index is invalid.
    pub fn remove(&mut self, idx: GenIdx) -> Option<T> {
        let entry = self.storage.get_mut(idx.index as usize)?;

        if entry.generation == idx.generation {
            self.free_list.push(idx.index);
            entry.item.take()
        } else {
            None
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            storage: &self.storage,
            current: 0,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        let len = self.storage.len();
        // SAFETY: self.storage is guaranteed to be non-null and valid.
        let start = unsafe { NonNull::new_unchecked(self.storage.as_mut_ptr()) };
        IterMut {
            start,
            current: 0,
            len,
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct Iter<'a, T> {
    storage: &'a Vec<GenEntry<T>>,
    current: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (GenIdx, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.storage.len() {
            let entry = &self.storage[self.current];
            self.current += 1;

            if let Some(item) = &entry.item {
                return Some((
                    GenIdx {
                        generation: entry.generation,
                        index: self.current as u32 - 1,
                    },
                    item,
                ));
            }
        }

        None
    }
}

pub struct IterMut<'a, T> {
    start: NonNull<GenEntry<T>>,
    current: usize,
    len: usize,
    // Ensure the mutable borrow of GenVec<T> is valid for the lifetime of the iterator
    _marker: std::marker::PhantomData<&'a mut T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (GenIdx, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let entry = loop {
            if self.current >= self.len {
                return None;
            }

            // SAFETY: The caller needs to ensure that the `start` is a valid pointer to the first
            // element of the vector and that the `length` is the length of the vector. It is
            // guaranteed that `self.current` is less than `self.len`, so the pointer arithmetic is
            // within bounds.
            let entry = unsafe { &mut *self.start.as_ptr().add(self.current) };

            match entry.item {
                Some(_) => break entry,
                None => self.current += 1,
            }
        };

        let idx = GenIdx {
            generation: entry.generation,
            index: self.current as u32,
        };
        let item = entry.item.as_mut().expect("item is None");

        self.current += 1;
        Some((idx, item))
    }
}
