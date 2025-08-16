use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

use crate::callable::{CallableFetch, CallableParam};

use super::App;

#[derive(Debug, Default)]
pub struct StateStore {
    states: HashMap<TypeId, AnyState>,
}

#[derive(Debug, Clone)]
pub struct AnyState {
    data: Arc<RwLock<Box<dyn Any + Send + Sync>>>,
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

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("state of type {0} not found")]
    NotFound(String),
}

impl StateStore {
    pub fn insert<T: Send + Sync + 'static>(&mut self, data: T) {
        self.states.insert(
            TypeId::of::<T>(),
            AnyState {
                data: Arc::new(RwLock::new(Box::new(data))),
            },
        );
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<State<T>> {
        self.states.get(&TypeId::of::<T>()).map(|state| State {
            inner: state.clone(),
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: 'static> State<T> {
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        RwLockReadGuard::map(self.inner.data.read().await, |inner| {
            inner
                .downcast_ref::<T>()
                .expect("failed to downcast state (is the state of the right type?)")
        })
    }

    pub async fn write(&self) -> RwLockMappedWriteGuard<'_, T> {
        RwLockWriteGuard::map(self.inner.data.write().await, |inner| {
            inner
                .downcast_mut::<T>()
                .expect("failed to downcast state (is the state of the right type?)")
        })
    }
}

impl<T: Send + Sync + 'static, Ctx: CallableFetch<App> + Send + Sync, Init: Send + Sync>
    CallableParam<Ctx, Init> for State<T>
{
    type Error = StateError;

    async fn extract(ctx: &mut Ctx, _init: &Init) -> Result<Self, Self::Error> {
        let state = ctx
            .fetch()
            .inner
            .states
            .get::<T>()
            .ok_or_else(|| StateError::NotFound(std::any::type_name::<T>().to_string()))?;

        Ok(state)
    }
}
