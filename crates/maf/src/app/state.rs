use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug, Default)]
pub struct StateStore {
    states: HashMap<TypeId, AnyState>,
}

#[derive(Debug, Clone)]
pub struct AnyState {
    data: Arc<RwLock<Box<dyn Any>>>,
}

/// [`State`] is a wrapper around shared data that can be accessed throughout the app **without
/// being synchronized to clients**.
///
/// If client synchronization is needed, use [`crate::Store`] instead.
#[derive(Debug, Clone)]
pub struct State<T> {
    inner: AnyState,
    _phantom: std::marker::PhantomData<T>,
}

impl StateStore {
    pub fn insert<T: 'static>(&mut self, data: T) {
        self.states.insert(
            TypeId::of::<T>(),
            AnyState {
                data: Arc::new(RwLock::new(Box::new(data))),
            },
        );
    }
}

impl<T: 'static> State<T> {
    pub async fn read(&self) -> RwLockReadGuard<T> {
        RwLockReadGuard::map(self.inner.data.read().await, |inner| {
            inner
                .downcast_ref::<T>()
                .expect("failed to downcast store (is the store of the right type?)")
        })
    }

    pub async fn write(&self) -> RwLockMappedWriteGuard<T> {
        RwLockWriteGuard::map(self.inner.data.write().await, |inner| {
            inner
                .downcast_mut::<T>()
                .expect("failed to downcast store (is the store of the right type?)")
        })
    }
}
