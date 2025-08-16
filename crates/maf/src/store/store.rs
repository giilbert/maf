use std::{
    any::{Any, TypeId},
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
};

#[cfg(feature = "typed")]
use schemars::{JsonSchema, SchemaGenerator};
use serde::Serialize;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    callable::{CallableFetch, CallableParam},
    App, User,
};

use super::change_detection::StoreMut;

#[derive(Clone)]
pub struct AnyStore {
    pub(crate) type_id: TypeId,
    pub(crate) key: StoreKey,
    pub(crate) dirty: Arc<AtomicBool>,
    pub(crate) data: Arc<RwLock<dyn Any + Send + Sync>>,
    pub(crate) serializer: Arc<
        dyn Fn(&dyn Any, &User) -> Result<serde_json::Value, StoreSerializeError>
            + Send
            + Sync
            + 'static,
    >,

    #[cfg(feature = "typed")]
    pub(crate) desc:
        Arc<dyn Fn(&mut SchemaGenerator) -> crate::typed::StoreDesc + Send + Sync + 'static>,
}

impl std::fmt::Debug for AnyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyStore")
            .field("key", &self.key)
            .field("dirty", &self.dirty)
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreSerializeError {
    #[error("failed to serialize store data: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// [`Store`] is a wrapper around shared data that can be accessed throughout the app and
/// **synchronized with connected clients**.
///
/// The data stored in [`Store`] must implement [`StoreData`], describing initialization and access
/// methods.
pub struct Store<T: StoreData> {
    app: App,
    inner: AnyStore,
    _phantom: std::marker::PhantomData<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreKey(Arc<str>);

/// Describes the data stored in a [`Store`].
pub trait StoreData: Send + Sync + 'static {
    #[cfg(not(feature = "typed"))]
    type Select<'this>: Serialize;
    #[cfg(feature = "typed")]
    type Select<'this>: Serialize + JsonSchema;

    fn name() -> impl AsRef<str> + Send {
        std::any::type_name::<Self>()
    }

    fn key() -> impl Into<StoreKey> {
        StoreKey::from(Self::name().as_ref())
    }

    #[allow(unused_variables)]
    fn select(&self, user: &User) -> Self::Select<'_>;

    fn init() -> Self;
}

impl AnyStore {
    pub fn new<T: StoreData>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            key: T::key().into(),
            dirty: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(T::init())),
            serializer: Arc::new(|data, user| {
                let data = data.downcast_ref::<T>().expect(&std::format!(
                    "store data is not of expected type {}",
                    std::any::type_name::<T>()
                ));

                serde_json::to_value(T::select(&data, user)).map_err(Into::into)
            }),
            #[cfg(feature = "typed")]
            desc: Arc::new(|generator| crate::typed::StoreDesc::new::<T>(generator)),
        }
    }
}

impl From<&str> for StoreKey {
    fn from(key: &str) -> Self {
        Self(Arc::from(key))
    }
}

impl AsRef<str> for StoreKey {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<T: StoreData> Store<T> {
    pub async fn new(app: App) -> Self {
        let key = T::key().into();
        let inner = app
            .inner
            .state
            .stores
            .read()
            .await
            .get(&key)
            .cloned()
            .expect("store not found");

        Store {
            app,
            inner,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        RwLockReadGuard::map(self.inner.data.read().await, |inner| {
            inner
                .downcast_ref::<T>()
                .expect("failed to downcast store (is the store of the right type?)")
        })
    }

    pub async fn write(&self) -> StoreMut<'_, T> {
        StoreMut::new(
            &self.app,
            &self.inner,
            RwLockWriteGuard::map(self.inner.data.write().await, |inner| {
                inner
                    .downcast_mut::<T>()
                    .expect("failed to downcast store (is the store of the right type?)")
            }),
        )
    }

    pub async fn flush(&self) {
        if self.inner.dirty.load(atomic::Ordering::Relaxed) {
            self.inner.dirty.store(false, atomic::Ordering::Relaxed);
        }
    }
}

impl<T: StoreData, Ctx: CallableFetch<App> + Send + Sync, Init: Send + Sync>
    CallableParam<Ctx, Init> for Store<T>
{
    type Error = std::convert::Infallible;

    async fn extract(ctx: &mut Ctx, _init: &Init) -> Result<Self, Self::Error> {
        let app = ctx.fetch();
        let key = T::key().into();

        let existing_store = app.inner.state.stores.read().await.get(&key).cloned();

        let store = match existing_store {
            Some(store) => store,
            None => {
                // Code is structured this way to avoid deadlocks when acquiring the read lock
                // and then trying to acquire the write lock.
                drop(existing_store);

                let store = AnyStore::new::<T>();

                app.inner
                    .state
                    .stores
                    .write()
                    .await
                    .insert(key, store.clone());

                store
            }
        };

        Ok(Store {
            app: app.clone(),
            inner: store,
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: StoreData> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            inner: self.inner.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}
