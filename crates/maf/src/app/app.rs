use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use tokio::sync::{
    mpsc::{self, error::TryRecvError},
    RwLock, RwLockReadGuard,
};
use uuid::Uuid;

use crate::{
    bindings::bindgen,
    channel::UntypedChannelBroadcast,
    packet::{ChannelSendRx, OneStoreUpdate, RxPacket, TxPacket},
    rpc::{models::TypedRpcRequestPacket, IntoRpcFunction, RpcError, RpcStore},
    store::{AnyStore, StoreKey},
    tasks::{self, Runtime},
    user::UserMessage,
    Channel, StoreData, User, UserListener,
};

use super::{
    background::{BackgroundFn, IntoBackgroundFn},
    on_connect::OnConnectFn,
    IntoOnConnect,
};

#[derive(Clone)]
pub struct App {
    pub(crate) inner: Arc<AppInner>,
}

pub struct AppInner {
    pub(crate) state: Arc<AppState>,
    pub(crate) rpc_functions: RpcStore,
    pub(crate) store_dirty_rx: RwLock<mpsc::Receiver<StoreKey>>,
    pub(crate) on_connect: Option<Arc<OnConnectFn>>,
    pub(crate) background: Option<Arc<BackgroundFn>>,
}

#[derive(Debug)]
pub struct AppState {
    pub(crate) users: RwLock<HashMap<Uuid, User>>,
    pub(crate) stores: RwLock<HashMap<StoreKey, AnyStore>>,
    pub(crate) store_dirty: mpsc::Sender<StoreKey>,
    pub(crate) channels: RwLock<HashMap<String, UntypedChannelBroadcast>>,
    pub(crate) user_rx_channels: RwLock<HashMap<(Uuid, String), UntypedChannelBroadcast>>,
}

#[derive(Default)]
pub struct AppBuilder {
    on_connect: Option<Arc<OnConnectFn>>,
    background: Option<Arc<BackgroundFn>>,
    rpc_functions: RpcStore,
    stores: HashMap<StoreKey, AnyStore>,
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }

    async fn handle_connections(self: Arc<Self>) -> Result<(), bindgen::ListenError> {
        let users = UserListener::new(self.inner.state.clone())?;

        // TODO: handle errors
        loop {
            let user = users.next().await?;
            self.inner
                .state
                .users
                .write()
                .await
                .insert(user.meta.id, user.clone());

            self.refresh_all_stores(&user).await.ok();

            // Listen for messages from the user and handle them
            let user_clone = user.clone();
            let app = self.clone();
            tasks::spawn(async move {
                user_clone
                    .handle_messages(app.clone())
                    .await
                    .expect("failed to handle user messages");

                // println!("user disconnected: {}", user_clone.meta.id());

                app.inner
                    .state
                    .users
                    .write()
                    .await
                    .remove(&user_clone.meta.id);
            });

            let app = self.clone();
            self.inner.on_connect.as_ref().map(|handler| {
                let handler = handler.clone();
                tasks::spawn(handler(&app, user));
            });
        }
    }

    async fn handle_channel_send(&self, user: &User, channel_data: ChannelSendRx) {
        // Send to general channels
        self.inner
            .state
            .channels
            .read()
            .await
            .get(&channel_data.channel)
            .map(|c| c.tx.send(channel_data.clone()));

        // Send to user-specific channels
        self.inner
            .state
            .user_rx_channels
            .read()
            .await
            .get(&(user.meta.id, channel_data.channel.clone()))
            .map(|c| c.tx.send(channel_data.clone()));
    }

    async fn handle_rpc(
        &self,
        user: &User,
        rpc_data: TypedRpcRequestPacket,
    ) -> Result<(), RpcError> {
        let res = self
            .inner
            .rpc_functions
            .handle_typed_rpc_request(self.clone(), user, rpc_data)
            .await?;

        match user.send(TxPacket::<()>::TypedRpcResponse(res)) {
            Ok(_) => {}
            Err(err) => {
                println!("failed to send rpc response: {err}");
            }
        }

        self.flush_all_store_changes().await?;

        Ok(())
    }

    pub(crate) async fn handle_message<'a>(&self, message: UserMessage<'a>) -> anyhow::Result<()> {
        match message.packet {
            RxPacket::ChannelSend(channel_data) => {
                self.handle_channel_send(&message.user, channel_data).await;
            }
            RxPacket::TypedRpcCall(rpc_data) => self.handle_rpc(&message.user, rpc_data).await?,
        }

        Ok(())
    }

    async fn flush_all_store_changes(&self) -> anyhow::Result<()> {
        let mut store_dirty_rx = self.inner.store_dirty_rx.write().await;

        loop {
            let store_key = match store_dirty_rx.try_recv() {
                Ok(store_key) => store_key,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    println!("store dirty channel disconnected");
                    return Ok(());
                }
            };

            self.flush_store_change(store_key).await?;
        }

        Ok(())
    }

    async fn flush_store_change(&self, store_key: StoreKey) -> anyhow::Result<()> {
        let store = RwLockReadGuard::try_map(self.inner.state.stores.read().await, |stores| {
            stores.get(&store_key)
        })
        .map_err(|_| anyhow::anyhow!("failed to get store"))?
        .clone();

        let serializer = store.serializer.clone();
        let data = store.data.read_owned().await;

        let serialized =
            serializer(&*data).map_err(|_| anyhow::anyhow!("failed to serialize store"))?;

        for (user_id, user) in self.inner.state.users.read().await.iter() {
            if let Err(e) = user.send(TxPacket::StoreUpdate(OneStoreUpdate {
                store: store_key.as_ref(),
                data: &serialized,
            })) {
                println!("failed to send store update to user {user_id}: {e}");
            }
        }

        Ok(())
    }

    async fn refresh_all_stores(&self, user: &User) -> anyhow::Result<()> {
        let stores = self.inner.state.stores.read().await;

        let mut data: Vec<(&StoreKey, serde_json::Value)> = Vec::with_capacity(stores.len());

        for (store_key, store) in stores.iter() {
            let serialized = (store.serializer)(&*store.data.read().await)
                .context("failed to serialize store")?;

            data.push((store_key, serialized));
        }

        user.send(TxPacket::ManyStoreUpdate::<serde_json::Value>(
            data.iter()
                .map(|(k, v)| OneStoreUpdate {
                    store: k.as_ref(),
                    data: v,
                })
                .collect(),
        ))
        .context("failed to send store update")?;

        Ok(())
    }

    async fn run_async(self) {
        let background = self
            .inner
            .background
            .as_ref()
            .map(|handler| tasks::spawn(handler(self.clone())));

        let app = Arc::new(self);
        app.handle_connections()
            .await
            .expect("failed to handle connections");

        if let Some(background) = background {
            background.await;
        }

        println!("run_async finished")
    }

    pub fn run(self) {
        tasks::spawn(self.run_async());
        Runtime::current().blocking_poll();
    }

    pub async fn user(&self, user_id: Uuid) -> Option<User> {
        self.inner.state.users.read().await.get(&user_id).cloned()
    }

    pub fn channel<T>(&self, name: impl ToString) -> Channel<T> {
        Channel::new(self.inner.state.clone(), name.to_string())
    }
}

impl AppBuilder {
    pub fn on_connect<P, R>(mut self, handler: impl IntoOnConnect<P, R>) -> Self {
        self.on_connect = Some(handler.into_on_connect());
        self
    }

    pub fn rpc<P, R>(mut self, path: impl ToString, handler: impl IntoRpcFunction<P, R>) -> Self {
        let path = path.to_string();
        self.rpc_functions
            .add_rpc_function(handler.into_rpc_function(path));
        self
    }

    pub fn background<T>(mut self, handler: impl IntoBackgroundFn<T>) -> Self {
        self.background = Some(handler.into_background_fn());
        self
    }

    /// Statically declare a store
    pub fn store<T: StoreData>(mut self) -> Self {
        self.stores.insert(T::key().into(), AnyStore::new::<T>());
        self
    }

    pub fn build(self) -> App {
        const STORE_UPDATE_LIMIT: usize = 10_000;

        let (store_dirty, store_dirty_rx) = mpsc::channel(STORE_UPDATE_LIMIT);

        let state = Arc::new(AppState {
            store_dirty,
            channels: Default::default(),
            stores: RwLock::new(self.stores),
            user_rx_channels: Default::default(),
            users: Default::default(),
        });

        let inner = AppInner {
            state,
            store_dirty_rx: RwLock::new(store_dirty_rx),
            rpc_functions: self.rpc_functions,
            on_connect: self.on_connect,
            background: self.background,
        };

        App {
            inner: Arc::new(inner),
        }
    }
}

#[macro_export]
macro_rules! register {
    ($func:ident) => {
        pub use $crate::bindings::bindgen::{
            self, __export_world_imports_cabi, _export_run_cabi, export,
        };

        pub struct GuestImpl {}

        impl bindgen::Guest for GuestImpl {
            fn run() -> Result<(), ()> {
                $crate::bindings::init_panic_hook();
                $crate::tasks::Runtime::new().global();
                let app = $func();
                app.run();
                Ok(())
            }
        }

        #[allow(unsafe_op_in_unsafe_fn)]
        export!(GuestImpl);
    };
}
