use std::{
    any::Any,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
};

use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    callable::{CallableFetch, CallableParam},
    App, User,
};

use super::change_detection::StoreMut;

#[derive(Clone)]
pub struct AnyStore {
    pub(crate) key: StoreKey,
    pub(crate) dirty: Arc<AtomicBool>,
    pub(crate) data: Arc<RwLock<dyn Any + Send + Sync>>,
    pub(crate) serializer: Arc<
        dyn Fn(&dyn Any, &User) -> Result<serde_json::Value, StoreSerializeError>
            + Send
            + Sync
            + 'static,
    >,
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
pub trait StoreData: 'static {
    type Data: Send + Sync + 'static;

    fn name() -> impl AsRef<str> + Send {
        std::any::type_name::<Self>()
    }

    fn key() -> impl Into<StoreKey> {
        StoreKey::from(Self::name().as_ref())
    }

    #[allow(unused_variables)]
    fn select(data: &Self::Data, user: &User) -> impl serde::Serialize {
        ()
    }

    fn init() -> Self::Data;
}

impl AnyStore {
    pub fn new<T: StoreData>() -> Self {
        Self {
            key: T::key().into(),
            dirty: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(T::init())),
            serializer: Arc::new(|data, user| {
                let data = data.downcast_ref::<T::Data>().expect(&std::format!(
                    "store data is not of expected type {}",
                    std::any::type_name::<T::Data>()
                ));

                serde_json::to_value(T::select(&data, user)).map_err(Into::into)
            }),
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
    pub async fn read(&self) -> RwLockReadGuard<T::Data> {
        RwLockReadGuard::map(self.inner.data.read().await, |inner| {
            inner
                .downcast_ref::<T::Data>()
                .expect("failed to downcast store (is the store of the right type?)")
        })
    }

    pub async fn write(&self) -> StoreMut<T::Data> {
        StoreMut::new(
            &self.app,
            &self.inner,
            RwLockWriteGuard::map(self.inner.data.write().await, |inner| {
                inner
                    .downcast_mut::<T::Data>()
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
