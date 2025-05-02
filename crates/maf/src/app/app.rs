use std::{cell::RefCell, collections::HashMap, sync::Arc};

use async_lock::RwLock;
use uuid::Uuid;

use crate::{
    channel::UntypedChannelBroadcast,
    packet::{ChannelSendRx, RxPacket, TxPacket},
    rpc::{models::TypedRpcRequestPacket, IntoRpcFunction, RpcStore},
    store::AnyStore,
    tasks::{self, Runtime},
    user::UserMessage,
    Channel, User, UserListener,
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
    pub(crate) on_connect: Option<Arc<OnConnectFn>>,
    pub(crate) background: Option<Arc<BackgroundFn>>,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub(crate) users: RwLock<HashMap<Uuid, User>>,
    pub(crate) stores: RwLock<HashMap<String, AnyStore>>,

    pub(crate) channels: RwLock<HashMap<String, UntypedChannelBroadcast>>,
    pub(crate) user_rx_channels: RwLock<HashMap<(Uuid, String), UntypedChannelBroadcast>>,
}

#[derive(Default)]
pub struct AppBuilder {
    on_connect: Option<Arc<OnConnectFn>>,
    background: Option<Arc<BackgroundFn>>,
    rpc_functions: RpcStore,
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }

    async fn handle_connections(self: Arc<Self>) -> anyhow::Result<()> {
        let users = UserListener::new(self.inner.state.clone())?;

        loop {
            let user = users.next().await?;
            self.inner
                .state
                .users
                .write()
                .await
                .insert(user.meta.id, user.clone());

            // Listen for messages from the user and handle them
            let user_clone = user.clone();
            let app = self.clone();
            tasks::spawn(async move {
                user_clone
                    .handle_messages(app)
                    .await
                    .expect("failed to handle user messages");
            });

            let app = self.clone();
            self.inner.on_connect.as_ref().map(|handler| {
                let handler = handler.clone();
                tasks::spawn(handler(&app, user));
            });
        }
    }

    async fn handle_channel_send(
        &self,
        user: &User,
        channel_data: ChannelSendRx,
    ) -> anyhow::Result<()> {
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

        Ok(())
    }

    async fn handle_rpc(&self, user: &User, rpc_data: TypedRpcRequestPacket) -> anyhow::Result<()> {
        let res = self
            .inner
            .rpc_functions
            .handle_typed_rpc_request(self.inner.state.clone(), rpc_data)
            .await?;

        user.send(TxPacket::<()>::TypedRpcResponse(res))?;

        Ok(())
    }

    pub(crate) async fn handle_message<'a>(&self, message: UserMessage<'a>) -> anyhow::Result<()> {
        match message.packet {
            RxPacket::ChannelSend(channel_data) => {
                self.handle_channel_send(&message.user, channel_data).await
            }
            RxPacket::TypedRpcCall(rpc_data) => self.handle_rpc(&message.user, rpc_data).await,
        }
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

    pub fn build(self) -> App {
        let state = Arc::new(AppState::default());

        let inner = AppInner {
            state,
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

        export!(GuestImpl);
    };
}
