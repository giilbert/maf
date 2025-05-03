use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{self, AtomicBool},
};

use tokio::sync::RwLockMappedWriteGuard;

use crate::App;

use super::{AnyStore, StoreKey};

pub struct StoreMut<'a, T> {
    app: &'a App,
    inner: &'a AnyStore,
    guard: RwLockMappedWriteGuard<'a, T>,
}

impl<'a, T> StoreMut<'a, T> {
    pub fn new(state: &'a App, inner: &'a AnyStore, guard: RwLockMappedWriteGuard<'a, T>) -> Self {
        Self {
            app: state,
            guard,
            inner,
        }
    }
}

impl<'a, T> Deref for StoreMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, T> DerefMut for StoreMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.dirty.store(true, atomic::Ordering::Relaxed);

        self.guard.deref_mut()
    }
}

impl<T> Drop for StoreMut<'_, T> {
    fn drop(&mut self) {
        if self.inner.dirty.swap(false, atomic::Ordering::Relaxed) {
            let app = self.app.clone();
            app.inner
                .state
                .store_dirty
                .try_send(self.inner.key.clone())
                .expect("failed to mark store as dirty: too many updates");
        }
    }
}
