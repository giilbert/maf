use core::panic;
use std::{
    any::{Any, TypeId},
    sync::{atomic::AtomicBool, Arc},
};

use mea::rwlock::{
    MappedRwLockReadGuard, OwnedMappedRwLockReadGuard, OwnedRwLockReadGuard, OwnedRwLockWriteGuard,
    RwLock, RwLockReadGuard, RwLockWriteGuard,
};
#[cfg(feature = "typed")]
use schemars::{JsonSchema, SchemaGenerator};
use serde::Serialize;

use crate::{
    callable::{CallableFetch, CallableParam, SupportsAsync},
    store::pointers::OwnedStoreWriteLock,
    App, User,
};

use super::pointers::StoreWriteLock;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoreId(pub(crate) TypeId);

impl StoreId {
    pub fn of<T: 'static>() -> Self {
        StoreId(TypeId::of::<T>())
    }
}

type AnyStoreSerializerFn = Arc<
    dyn Fn(&dyn Any, &User) -> Result<serde_json::Value, StoreSerializeError>
        + Send
        + Sync
        + 'static,
>;

/// A type-erased store that can hold any data implementing [`StoreData`].
#[derive(Clone)]
pub struct AnyStore {
    /// A unique identifier for the store. This is the [`TypeId`] of the store's data type.
    pub(crate) id: StoreId,
    /// The name of the store, used to identify it to clients.
    pub(crate) name: Arc<str>,
    /// Indicates whether the store's data has been modified and needs to be synchronized with
    /// clients.
    pub(crate) dirty: Arc<AtomicBool>,
    /// The store's data is stored as a type-erased [`RwLock`].
    pub(crate) data: Arc<RwLock<dyn Any + Send + Sync>>,
    /// Function to serialize the store's data for a given user. If serialization fails, returns a
    /// [`StoreSerializeError`].
    pub(crate) serializer: AnyStoreSerializerFn,

    #[cfg(feature = "typed")]
    pub(crate) desc:
        Arc<dyn Fn(&mut SchemaGenerator) -> crate::typed::StoreDesc + Send + Sync + 'static>,
}

impl std::fmt::Debug for AnyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyStore")
            .field("id", &self.id)
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
///
/// ## Usage
/// ```rust
/// use maf::prelude::*;
///
/// // [1]: Define struct to hold store data
/// struct Counter {
///     count: u32,
/// }
///
/// // [2]: Implement StoreData for the struct
/// impl StoreData for Counter {
///     // ... see StoreData docs for more info ...
/// }
///
/// // [3]: Use your store in a callable function
/// fn increment_counter(mut counter: StoreMut<Counter>) {
///     counter.count += 1;
/// }
///
/// fn build() -> App {
///     App::builder()
///         .rpc("increment_counter", increment_counter)
///         // [4]: Create and register the store in your app
///         .store::<Counter>()
///         .build()
/// }
///
/// maf::register!(build);
/// ```
///
/// ## [`super::StoreRef`] and [`super::StoreMut`]
/// [`super::StoreRef`] and [`super::StoreMut`] provide read-only and mutable access to the store's
/// data within callable functions (e.g. RPC methods, `on_connect` handlers, etc). When using these
/// types, no async functions or locking is needed--MAF handles acquiring and releasing locks for
/// you. **[super::StoreRef] and [super::StoreMut]** cannot be used in async callables as an effort
/// to prevent lock contention and deadlocks.
///
/// ```rust
/// use maf::prelude::*;
///
/// struct Counter {
///     count: u32,
/// }
///
/// impl StoreData for Counter {
///     // ...
/// }
///
/// // No async function or locking is needed!
/// fn add_count(mut counter: StoreMut<Counter>) {
///     counter.count += 1;
/// }
///
/// // ...
/// ```
///
/// Under the hood, [`super::StoreRef`] acquires a read lock on the store's data for the duration of
/// the callable, while [`super::StoreMut`] acquires a write lock.
///
/// ## 🔗🔗🔗 Beware of Lock Contention!
/// [`Store`] uses [`RwLock`] internally to allow concurrent read access and exclusive write access.
/// However, if a callable function (e.g. an RPC method or an `on_connect` handler) holds a lock
/// on a store and then awaits an async operation, it may lead to deadlocks and other slowness if
/// other callables try to access the same store.
///
/// As an example of what **NOT** to do:
/// ```rust
/// struct Game {
///     is_powerup_active: bool,
///     health: u32,
/// }
///
/// impl StoreData for Game {
///     // ... implementation left out for brevity ...
/// }
///
/// /// Set `is_powerup_active` to true for 30 seconds, then back to false.
/// async fn bad_trigger_powerup(game: Store<Game>) {
///     // Acquires a write lock on the store's data
///     let mut game_data = game.write().await;
///     game_data.is_powerup_active = true;
///     
///     // The powerup lasts for 30 seconds
///     tasks::sleep(std::time::Duration::from_secs(30)).await;
///     game_data.is_powerup_active = false;
/// }
///
/// async fn heal_player(game: Store<Game>) {
///     // This will wait for a very long time if called while `bad_trigger_powerup` is sleeping!
///     let mut game_data = game.write().await;
///     // NOTE: Although this example uses a write lock, even a read lock or StoreRef/StoreMut
///     // would be blocked while `bad_trigger_powerup` holds the write lock.
///     game_data.health += 10;
/// }
///
/// // ... register RPCs, stores, etc.
/// ```
///
/// The problem with the above example is that `bad_trigger_powerup` holds a write lock on the
/// store's data while it awaits the 30-second sleep. During this time, any other callable that
/// tries to access the store (like `heal_player`) will be blocked, leading to potential deadlocks
/// or significant delays. To avoid this, ensure that locks are held for the shortest
/// time possible and avoid awaiting while holding locks.
///
/// A better approach would be to minimize the locked section:
/// ```rust
/// async fn good_trigger_powerup(game: Store<Game>) {
///     game_data.write().await.is_powerup_active = true;
///     tasks::sleep(std::time::Duration::from_secs(30)).await;
///     game_data.write().await.is_powerup_active = false;
/// }
/// ```
///
/// In doing this, the lock is only held while updating the `is_powerup_active` field, and not
/// during the sleep period, allowing other callables to access the store without unnecessary
/// delays.
///
/// Another strategy is to use interior mutability patterns within the store's data (e.g., using
/// `AtomicBool`, `Mutex`, etc.) to allow safe concurrent modifications without holding locks. A
/// third strategy is to split commonly locked/updating data into separate stores to reduce the
/// amount of sharing that needs to occur.
pub struct Store<T: StoreData> {
    app: App,
    pub(crate) inner: AnyStore,
    _phantom: std::marker::PhantomData<T>,
}

/// Describes the data stored in a [`Store`].
pub trait StoreData: Send + Sync + 'static {
    #[cfg(not(feature = "typed"))]
    /// The type of data selected to be serialized and sent to clients. This type must implement
    /// [`serde::Serialize`].
    type Select<'this>: Serialize;
    #[cfg(feature = "typed")]
    /// The type of data selected to be serialized and sent to clients. This type must implement
    /// [`serde::Serialize`] and [`schemars::JsonSchema`].
    type Select<'this>: Serialize + JsonSchema;

    /// Initializes the store data with a default value.
    fn init() -> Self;

    /// Returns the name of the store to be used by clients to identify it.
    ///
    /// If not specified, the default is the Rust type name of `Self`.
    fn name() -> impl AsRef<str> + Send {
        std::any::type_name::<Self>()
    }

    /// Selects the portion of the store data to be serialized and sent to the given user.
    ///
    /// The lifetime `'this` enables returning references into `self`.
    ///
    /// ## Example
    /// ```rust
    /// use maf::prelude::*;
    ///
    /// struct Message {
    ///     text: String,
    ///     secret: String,
    /// }
    ///
    /// impl StoreData for Message {
    ///     // The 'this lifetime refers to the lifetime of the data being selected and it allows
    ///     // zero-copy serialization by returning references into self. This is an optional
    ///     // feature--you can also return owned data if preferred.
    ///     type Select<'this> = &'this String;
    ///
    ///     fn init() -> Self {
    ///         Message {
    ///             text: "Hello, world!".to_string(),
    ///             secret: "This is a secret.".to_string(),
    ///         }
    ///     }
    ///
    ///     fn select(&self, user: &User) -> Self::Select<'_> {
    ///         // Send only the public text, not the secret to the client
    ///         &self.text
    ///     }
    ///
    ///     fn name() -> impl AsRef<str> {
    ///         "message"
    ///     }
    /// }
    ///
    /// ```
    #[allow(unused_variables)]
    fn select(&self, user: &User) -> Self::Select<'_>;
}

impl AnyStore {
    pub fn new<T: StoreData>() -> Self {
        Self {
            id: StoreId::of::<T>(),
            name: Arc::from(T::name().as_ref()),
            dirty: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(T::init())),
            serializer: Arc::new(|data, user| {
                let data = data.downcast_ref::<T>().unwrap_or_else(|| {
                    panic!(
                        "store data is not of expected type {}",
                        std::any::type_name::<T>()
                    )
                });

                serde_json::to_value(T::select(data, user)).map_err(Into::into)
            }),
            #[cfg(feature = "typed")]
            desc: Arc::new(|generator| crate::typed::StoreDesc::new::<T>(generator)),
        }
    }
}

impl<T: StoreData> Store<T> {
    /// Creates a new [`Store<T>`] instance for the given app.
    ///
    /// ## Panics
    /// The type of the store should have already been registered with the app using
    /// [`crate::AppBuilder::store`]. If the store is not found, this function will panic.
    pub async fn new(app: App) -> Self {
        let id = StoreId::of::<T>();
        let inner = app
            .inner
            .state
            .stores
            .read()
            .await
            .get(&id)
            .cloned()
            .expect("store not found");

        Store {
            app,
            inner,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Acquires a read lock on the store's data. Multiple read locks can be held concurrently.
    ///
    /// This creates a lock with a lifetime tied to the `Store` instance. If you need an owned lock
    /// that can outlive the `Store`, consider using [`Store::read_owned`].
    pub async fn read(&self) -> MappedRwLockReadGuard<'_, T> {
        RwLockReadGuard::map(self.inner.data.read().await, |inner| {
            inner
                .downcast_ref::<T>()
                .expect("failed to downcast store (is the store of the right type?)")
        })
    }

    /// Acquires a write lock on the store's data. Only one write lock can be held at a time, and
    /// no read locks can be held while a write lock is active.
    ///
    /// This creates a lock with a lifetime tied to the `Store` instance. If you need an owned lock
    /// that can outlive the `Store`, consider using [`Store::write_owned`].
    pub async fn write(&self) -> StoreWriteLock<'_, T> {
        StoreWriteLock::new(
            &self.app,
            &self.inner,
            RwLockWriteGuard::map(self.inner.data.write().await, |inner| {
                inner
                    .downcast_mut::<T>()
                    .expect("failed to downcast store (is the store of the right type?)")
            }),
        )
    }

    /// Acquires an owned read lock on the store's data. Multiple read locks can be held
    /// concurrently.
    ///
    /// This creates a lock that is owned and can outlive the `Store` instance.
    pub async fn read_owned(&self) -> OwnedMappedRwLockReadGuard<dyn Any + Send + Sync, T> {
        OwnedRwLockReadGuard::map(self.inner.data.clone().read_owned().await, |inner| {
            inner
                .downcast_ref::<T>()
                .expect("failed to downcast store (is the store of the right type?)")
        })
    }

    /// Acquires an owned write lock on the store's data. Only one write lock can be held at a
    /// time, and no read locks can be held while a write lock is active.
    ///
    /// This creates a lock that is owned and can outlive the `Store` instance.
    pub async fn write_owned(&self) -> OwnedStoreWriteLock<T> {
        OwnedStoreWriteLock {
            app: self.app.clone(),
            store: self.clone(),
            guard: OwnedRwLockWriteGuard::map(
                self.inner.data.clone().write_owned().await,
                |inner| {
                    inner
                        .downcast_mut::<T>()
                        .expect("failed to downcast store (is the store of the right type?)")
                },
            ),
        }
    }
}

/// Implement [`CallableParam`] for [`Store<T>`] to allow it to be used as a parameter in **async
/// AND sync** callables (e.g. [`crate::rpc`] methods, [`crate::AppBuilder::on_connect`] handlers,
/// etc).
impl<T: StoreData, Ctx: CallableFetch<App> + Send + Sync, Init: Send + Sync>
    CallableParam<Ctx, Init> for Store<T>
{
    type Error = std::convert::Infallible;

    async fn extract(ctx: &mut Ctx, _init: &Init) -> Result<Self, Self::Error> {
        let app = ctx.fetch();

        let id = StoreId::of::<T>();
        let existing_store = app.inner.state.stores.read().await.get(&id).cloned();

        let store = match existing_store {
            Some(store) => store,
            None => panic!("store not found when trying to extract parameter: {:?}", id),
        };

        Ok(Store {
            app: app.clone(),
            inner: store,
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: StoreData> SupportsAsync for Store<T> {}

impl<T: StoreData> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            inner: self.inner.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}
