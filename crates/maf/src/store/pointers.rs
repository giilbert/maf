use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::atomic,
};

use tokio::sync::{OwnedRwLockMappedWriteGuard, OwnedRwLockReadGuard, RwLockMappedWriteGuard};

use crate::{
    callable::{CallableFetch, CallableParam},
    App, Store, StoreData,
};

use super::AnyStore;

/// A mutable reference to a store's data with automatic dirty tracking.
///
/// This is used to modify the store's data and mark the store as dirty, which will queue it for
/// update to clients.
///
/// ## As a [`CallableParam`]
/// [`StoreMut`] can be used as a parameter in callables, allowing direct modification of store data
/// within callable functions.
pub struct StoreWriteLock<'a, T: StoreData> {
    app: &'a App,
    inner: &'a AnyStore,
    guard: RwLockMappedWriteGuard<'a, T>,
}

impl<'a, T: StoreData> StoreWriteLock<'a, T> {
    pub fn new(app: &'a App, inner: &'a AnyStore, guard: RwLockMappedWriteGuard<'a, T>) -> Self {
        Self { app, guard, inner }
    }
}

impl<'a, T: StoreData> Deref for StoreWriteLock<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, T: StoreData> DerefMut for StoreWriteLock<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.dirty.store(true, atomic::Ordering::Relaxed);
        self.guard.deref_mut()
    }
}

impl<T: StoreData> Drop for StoreWriteLock<'_, T> {
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

/// An owned mutable reference to a store's data with automatic dirty tracking.
pub struct OwnedStoreWriteLock<T: StoreData> {
    pub(crate) app: App,
    pub(crate) store: Store<T>,
    pub(crate) guard: OwnedRwLockMappedWriteGuard<dyn Any + Send + Sync, T>,
}

impl<T: StoreData> Deref for OwnedStoreWriteLock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<T: StoreData> DerefMut for OwnedStoreWriteLock<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.store
            .inner
            .dirty
            .store(true, atomic::Ordering::Relaxed);
        self.guard.deref_mut()
    }
}

impl<T: StoreData> Drop for OwnedStoreWriteLock<T> {
    fn drop(&mut self) {
        // TODO: Is there a race condition here?
        if self
            .store
            .inner
            .dirty
            .swap(false, atomic::Ordering::Relaxed)
        {
            let app = self.app.clone();
            app.inner
                .state
                .store_dirty
                .try_send(self.store.inner.key.clone())
                .expect("failed to mark store as dirty: too many updates");
        }
    }
}

/// A mutable reference to a store's data used in callables.
pub struct StoreMut<T: StoreData> {
    inner: OwnedStoreWriteLock<T>,
}

impl<T: StoreData, Ctx: CallableFetch<App>, Init: Send + Sync> CallableParam<Ctx, Init>
    for StoreMut<T>
{
    type Error = std::convert::Infallible;

    async fn extract(ctx: &mut Ctx, _init: &Init) -> Result<Self, Self::Error> {
        let app = ctx.fetch();
        let store = Store::<T>::new(app.clone()).await;
        let guard = store.write_owned().await;
        Ok(StoreMut { inner: guard })
    }
}

/// A read-only reference to a store's data used in callables.
pub struct StoreRef<T: StoreData> {
    inner: OwnedRwLockReadGuard<dyn Any + Send + Sync, T>,
}

impl<T: StoreData, Ctx: CallableFetch<App>, Init: Send + Sync> CallableParam<Ctx, Init>
    for StoreRef<T>
{
    type Error = std::convert::Infallible;

    async fn extract(ctx: &mut Ctx, _init: &Init) -> Result<Self, Self::Error> {
        let app = ctx.fetch();
        let store = Store::<T>::new(app.clone()).await;
        let guard = store.read_owned().await;
        Ok(StoreRef { inner: guard })
    }
}

impl<T: StoreData> Deref for StoreRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: StoreData> Deref for StoreMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: StoreData> DerefMut for StoreMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}
