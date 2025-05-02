use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

use crate::{app::AppState, FromRequest, RpcRequest};

#[derive(Debug, Clone)]
pub struct AnyStore {
    pub(crate) name: String,
    pub(crate) data: Arc<RwLock<dyn std::any::Any + Send + Sync>>,
}

pub struct Store<T: StoreData> {
    inner: AnyStore,
    _phantom: std::marker::PhantomData<T>,
}

pub trait StoreData: 'static {
    type Data: Serialize + DeserializeOwned + Send + Sync;

    fn name() -> impl AsRef<str> + Send {
        std::any::type_name::<Self>()
    }

    fn key() -> impl AsRef<str> + Send {
        Self::name()
    }

    fn init() -> Self::Data;
}

impl AnyStore {
    pub fn new<T: StoreData>() -> Self {
        Self {
            name: T::name().as_ref().to_string(),
            data: Arc::new(RwLock::new(T::init())),
        }
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

    pub async fn write(&self) -> RwLockMappedWriteGuard<T::Data> {
        RwLockWriteGuard::map(self.inner.data.write().await, |inner| {
            inner
                .downcast_mut::<T::Data>()
                .expect("failed to downcast store (is the store of the right type?)")
        })
    }
}

impl<T: StoreData> FromRequest for Store<T> {
    async fn from_request(state: &AppState, _request: &mut RpcRequest) -> anyhow::Result<Self> {
        let key = T::key();

        let existing_store = state.stores.read().await.get(key.as_ref()).cloned();
        let store = match existing_store {
            Some(store) => store,
            None => {
                // Code is structured this way to avoid deadlocks when acquiring the read lock
                // and then trying to acquire the write lock.
                drop(existing_store);

                let store = AnyStore::new::<T>();

                println!("here");
                state
                    .stores
                    .write()
                    .await
                    .insert(key.as_ref().to_string(), store.clone());

                println!("there");

                store
            }
        };

        Ok(Store {
            inner: store,
            _phantom: std::marker::PhantomData,
        })
    }
}
