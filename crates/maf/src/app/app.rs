use std::{collections::HashMap, sync::Arc};

use async_lock::RwLock;
use uuid::Uuid;

use crate::{
    bindings::bindgen::ListenError,
    channel::UntypedChannelBroadcast,
    packet::RxPacket,
    rpc::{IntoRpcFunction, RpcStore},
    tasks::{self, Runtime},
    user::UserMessage,
    Channel, User, UserListener,
};

use super::{on_connect::OnConnectFn, IntoOnConnect};

#[derive(Clone)]
pub struct App {
    pub(crate) inner: Arc<AppInner>,
}

pub struct AppInner {
    pub(crate) state: Arc<AppState>,
    pub(crate) rpc_functions: RpcStore,
    pub(crate) on_connect: Option<Arc<OnConnectFn>>,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub(crate) users: RwLock<HashMap<Uuid, User>>,
    pub(crate) channels: RwLock<HashMap<String, UntypedChannelBroadcast>>,
}

#[derive(Default)]
pub struct AppBuilder {
    on_connect: Option<Arc<OnConnectFn>>,
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

            let app = self.clone();
            let user_clone = user.clone();

            // Listen for messages from the user and handle them
            tasks::spawn(async move {
                let messages = user_clone.listen_messages()?;

                loop {
                    let message = match messages.next().await {
                        Ok(message) => message,
                        Err(e) => {
                            if e.downcast_ref::<ListenError>()
                                .map(|e| matches!(e, ListenError::Closed))
                                .unwrap_or(false)
                            {
                                break;
                            } else {
                                return Err(e);
                            }
                        }
                    };

                    app.handle_message(message).await?;
                }

                Ok::<_, anyhow::Error>(())
            });

            let app = self.clone();
            self.inner.on_connect.as_ref().map(|handler| {
                let handler = handler.clone();
                tasks::spawn(handler(&app, user));
            });
        }
    }

    async fn handle_message<'a>(&self, message: UserMessage<'a>) -> anyhow::Result<()> {
        match message.packet {
            RxPacket::ChannelSend(channel_data) => {
                self.inner
                    .state
                    .channels
                    .read()
                    .await
                    .get(&channel_data.channel)
                    .map(|channel| {
                        channel
                            .tx
                            .send(channel_data)
                            .expect("failed to send message");
                    });
            }
        }

        Ok(())
    }

    async fn run_async(self) {
        let app = Arc::new(self);
        tasks::spawn(app.handle_connections()).on_finish(|e| {
            panic!("failed to run app: {:?}", e);
        });
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

    pub fn rpc<R, P>(mut self, path: impl ToString, handler: impl IntoRpcFunction<R, P>) -> Self {
        let path = path.to_string();
        self.rpc_functions
            .add_rpc_function(handler.into_rpc_function(path));
        self
    }

    pub fn build(self) -> App {
        let state = Arc::new(AppState::default());

        let inner = AppInner {
            state,
            rpc_functions: self.rpc_functions,
            on_connect: self.on_connect,
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
