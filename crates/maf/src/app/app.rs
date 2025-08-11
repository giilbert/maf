use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Context;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{
    mpsc::{self, error::TryRecvError},
    RwLock, RwLockReadGuard,
};
use uuid::Uuid;

use crate::{
    app::{background::BackgroundFnContext, hooks::HooksListener},
    bindings::bindgen,
    callable::{AnyCallable, IntoCallable},
    channel::UntypedChannelBroadcast,
    packet::{Borwned, ChannelSendRx, OneStoreUpdate, RxPacket, TxPacket},
    rpc::{
        models::{TypedRpcRequestPacket, TypedRpcResponsePacket},
        RpcError, RpcRequestContext, RpcRequestInit, RpcStore,
    },
    store::{
        AnySelect, AnyStore, GetParamSelectDependencies, SelectContext, SelectDependencyType,
        SelectKey, StoreKey,
    },
    tasks::{self, Runtime},
    user::UserMessage,
    Channel, RpcFunction, StoreData, User, UserListener,
};

use super::{
    background::{BackgroundFn, BackgroundFnError},
    hooks::{HookContext, HookError, HookFunction, HookInit, HookResponse, HookStore},
    on_connect_disconnect::{
        OnConnectDiconnectContext, OnConnectDisconnectError, OnConnectDisconnectFn,
    },
    state::StateStore,
};

#[derive(Clone)]
pub struct App {
    pub(crate) inner: Arc<AppInner>,
}

pub struct AppInner {
    pub(crate) state: Arc<AppState>,
    pub(crate) rpc_functions: RpcStore,
    pub(crate) states: StateStore,
    pub(crate) hooks: HookStore,
    pub(crate) store_dirty_rx: RwLock<mpsc::Receiver<StoreKey>>,
    pub(crate) on_connect: Option<Arc<OnConnectDisconnectFn>>,
    pub(crate) on_disconnect: Option<Arc<OnConnectDisconnectFn>>,
    pub(crate) background: Option<Arc<BackgroundFn>>,
    pub(crate) selects: HashMap<SelectKey, AnySelect>,
    pub(crate) select_dependencies: HashMap<TypeId, HashSet<SelectKey>>,
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
    on_connect: Option<Arc<OnConnectDisconnectFn>>,
    on_disconnect: Option<Arc<OnConnectDisconnectFn>>,
    background: Option<Arc<BackgroundFn>>,
    rpc_functions: RpcStore,
    states: StateStore,
    hooks: HookStore,
    stores: HashMap<StoreKey, AnyStore>,
    selects: HashMap<SelectKey, AnySelect>,
    /// Used to track what selects each store affects (maps store type id to the names of selects)
    ///
    /// TODO: refactor into separate storage
    select_dependencies: HashMap<TypeId, HashSet<SelectKey>>,
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }

    async fn handle_connections(self) -> Result<(), bindgen::ListenError> {
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
            let on_disconnect = self.inner.on_disconnect.clone();
            let on_connect = self.inner.on_connect.clone();
            tasks::spawn(async move {
                match on_connect.as_ref() {
                    Some(handler) => {
                        let handler = handler.clone();
                        if let Err(e) = handler(OnConnectDiconnectContext {
                            app: app.clone(),
                            user: user_clone.clone(),
                        })
                        .await
                        {
                            println!("failed to run on_connect handler: {e}");
                        }

                        app.flush_all_store_changes().await.ok();
                    }
                    None => (),
                }

                user_clone
                    .handle_messages(app.clone())
                    .await
                    .expect("failed to handle user messages");

                // println!("user disconnected: {}", user_clone.meta.id());

                match on_disconnect.as_ref() {
                    Some(handler) => {
                        let handler = handler.clone();
                        if let Err(e) = handler(OnConnectDiconnectContext {
                            app: app.clone(),
                            user: user_clone.clone(),
                        })
                        .await
                        {
                            println!("failed to run on_disconnect handler: {e}");
                        }

                        let _ = app.flush_all_store_changes().await;
                    }
                    None => {}
                }

                app.inner
                    .state
                    .users
                    .write()
                    .await
                    .remove(&user_clone.meta.id);
            });
        }
    }

    async fn handle_hook_requests(self) -> Result<(), bindgen::ListenError> {
        let hooks = HooksListener::new(self.inner.state.clone())?;

        loop {
            let request = hooks.next().await?;
            let app = self.clone();
            tasks::spawn(async move {
                if let Err(e) = app
                    .inner
                    .hooks
                    .handle_hook_request(app.clone(), request)
                    .await
                {
                    println!("failed to handle hook request: {e}");
                }
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

    async fn compute_select_contents(&self, name: &str, user: User) -> anyhow::Result<Value> {
        let any_select = self
            .inner
            .selects
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("select not found: {name}"))?;

        let value = (any_select.select)(SelectContext {
            app: self.clone(),
            user,
        })
        .await
        .map_err(|e| anyhow::anyhow!("failed to compute select contents for {name}: {e}"))?;

        Ok(value)
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

        let select_dependencies = self.inner.select_dependencies.get(&store.type_id);

        for (_user_id, user) in self.inner.state.users.read().await.iter() {
            let mut updates: Vec<OneStoreUpdate<Value>> = vec![];

            let serialized = serializer(&*data, &user)
                .map_err(|_| anyhow::anyhow!("failed to serialize store"))?;

            updates.push(OneStoreUpdate {
                store: store_key.as_ref(),
                data: Borwned::Borrowed(&serialized),
            });

            // Refresh any selects that depend on this store
            if let Some(selects) = select_dependencies {
                for select_name in selects {
                    let value = self
                        .compute_select_contents(&select_name, user.clone())
                        .await?;

                    updates.push(OneStoreUpdate {
                        store: select_name.as_ref(),
                        data: Borwned::Owned(value),
                    });
                }
            }

            user.send(TxPacket::ManyStoreUpdate::<()>(updates)).ok();
        }

        Ok(())
    }

    // TODO: allow users to subscribe to stores instead of sending updates optimistically
    async fn refresh_all_stores(&self, user: &User) -> anyhow::Result<()> {
        let stores = self.inner.state.stores.read().await;

        let mut data: Vec<(&str, serde_json::Value)> = Vec::with_capacity(stores.len());

        for (store_key, store) in stores.iter() {
            let serialized = (store.serializer)(&*store.data.read().await, user)
                .context("failed to serialize store")?;

            data.push((store_key.as_ref(), serialized));
        }

        // Also compute every select
        for (select_name, ..) in self.inner.selects.iter() {
            let value = self
                .compute_select_contents(select_name, user.clone())
                .await?;

            data.push((select_name, value));
        }

        user.send(TxPacket::ManyStoreUpdate::<serde_json::Value>(
            data.iter()
                .map(|(k, v)| OneStoreUpdate {
                    store: k.as_ref(),
                    data: Borwned::Borrowed(v),
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
            .map(|handler| tasks::spawn(handler(BackgroundFnContext { app: self.clone() })));

        let app = self.clone();

        tasks::spawn(async move {
            if let Err(e) = app.handle_hook_requests().await {
                println!("failed to handle hook requests: {e}");
            }
        });

        self.handle_connections()
            .await
            .expect("failed to handle connections");

        if let Some(background) = background {
            if let Err(e) = background.await {
                println!("background task failed: {e}");
            }
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
    /// Register a function to run when a user connects. To get the user object, use the [`User`]
    /// struct as a parameter.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use maf::*;
    ///
    /// fn on_connect(user: User) {
    ///     println!("user connected! id: {}", user.meta.id());
    /// }
    ///
    /// fn build() -> App {
    ///    App::builder()
    ///        .on_connect(on_connect)
    ///        .build()
    /// }
    /// ```
    pub fn on_connect<Params, Handler, const IS_ASYNC: bool>(mut self, handler: Handler) -> Self
    where
        Handler: IntoCallable<
            OnConnectDiconnectContext,
            Params,
            (),
            OnConnectDisconnectError,
            (),
            IS_ASYNC,
        >,
    {
        self.on_connect = Some(handler.into_callable(()).into());
        self
    }

    /// Register a function to run when a user disconnects. To get the user object, use the [`User`]
    /// struct as a parameter.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use maf::*;
    ///
    /// fn on_disconnect(user: User) {
    ///     println!("user disconnected! id: {}", user.meta.id());
    /// }
    ///
    /// fn build() -> App {
    ///    App::builder()
    ///        .on_disconnect(on_disconnect)
    ///        .build()
    /// }
    /// ```
    pub fn on_disconnect<Params, Handler, const IS_ASYNC: bool>(mut self, handler: Handler) -> Self
    where
        Handler: IntoCallable<
            OnConnectDiconnectContext,
            Params,
            (),
            OnConnectDisconnectError,
            (),
            IS_ASYNC,
        >,
    {
        self.on_disconnect = Some(handler.into_callable(()).into());
        self
    }

    /// Register a new RPC method.
    ///
    /// See [`crate::rpc`] module-level documentation for more details on RPCs.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use maf::*;
    ///
    /// struct CounterStore;
    ///
    /// impl StoreData for CounterStore {
    ///     type Data = i32;
    ///
    ///     fn init() -> Self::Data {
    ///         0
    ///     }
    /// }
    ///
    /// async fn add(Params(count): Params<i32>, store: Store<CounterStore>) -> i32 {
    ///     let mut data = store.write().await;
    ///     *data += count;
    ///     println!("incremented counter by {count}. new value: {}", &*data);
    ///     *data
    /// }
    ///
    /// fn build() -> App {
    ///     App::builder()
    ///         .rpc("add", add)
    ///         .store::<CounterStore>()
    ///         .build()
    /// }
    /// ```
    pub fn rpc<
        Params,
        Return,
        const IS_ASYNC: bool,
        #[cfg(feature = "typed")] TypedParams,
        #[cfg(feature = "typed")] TypedReturn,
        #[cfg(feature = "typed")] const TYPED_IS_ASYNC: bool,
        #[cfg(feature = "typed")] const TYPED_IS_RESULT: bool,
        #[cfg(feature = "typed")] Handler: IntoCallable<RpcRequestContext, Params, Return, RpcError, RpcRequestInit, IS_ASYNC>
            + crate::typed::ExtractRpcDesc<TypedParams, TypedReturn, TYPED_IS_ASYNC, TYPED_IS_RESULT>,
    >(
        mut self,
        method: impl ToString,
        #[cfg(feature = "typed")] handler: Handler,
        #[cfg(not(feature = "typed"))] handler: impl IntoCallable<
            RpcRequestContext,
            Params,
            Return,
            RpcError,
            RpcRequestInit,
            IS_ASYNC,
        >,
    ) -> Self
    where
        Return: Serialize + 'static,
    {
        use std::any::Any;

        let method = method.to_string();
        let callable: Arc<AnyCallable<RpcRequestContext, Return, RpcError>> =
            Arc::from(handler.into_callable(RpcRequestInit {
                method: method.clone(),
            }));

        self.rpc_functions.add_rpc_function(RpcFunction {
            type_id: handler.type_id(),
            method: method.clone(),
            handler: Box::new(move |ctx| {
                let callable = callable.clone();

                Box::pin(async move {
                    let id = ctx.request.id;
                    let result = callable(ctx).await?;

                    Ok(TypedRpcResponsePacket {
                        id,
                        result: serde_json::to_value(result)?,
                    })
                })
            }),
            #[cfg(feature = "typed")]
            desc: Handler::extract(method.clone()),
        });
        self
    }

    /// Register a task to run in the background.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use maf::*;
    ///
    /// async fn background() {
    ///     loop {
    ///         tasks::sleep(std::time::Duration::from_secs(1)).await;
    ///         println!("Hello from the background!");  
    ///     }
    /// }
    ///
    /// fn build() -> App {
    ///     App::builder()
    ///         .background(background)
    ///         .build()
    /// }
    /// ```
    pub fn background<Params, Handler, const IS_ASYNC: bool>(mut self, handler: Handler) -> Self
    where
        Handler: IntoCallable<BackgroundFnContext, Params, (), BackgroundFnError, (), IS_ASYNC>,
    {
        self.background = Some(handler.into_callable(()).into());
        self
    }

    /// Statically declare a store, initializing it with the default value.
    pub fn store<T: StoreData>(mut self) -> Self {
        self.stores.insert(T::key().into(), AnyStore::new::<T>());
        self
    }

    /// Register a store where its contents are derived with the provided function.
    pub fn select<
        Name: ToString,
        Params,
        Ret,
        Handler,
        const IS_ASYNC: bool,
        const N_PARAMS: usize,
    >(
        mut self,
        name: Name,
        handler: Handler,
    ) -> Self
    where
        Handler: IntoCallable<SelectContext, Params, Ret, std::convert::Infallible, (), IS_ASYNC>,
        Params: GetParamSelectDependencies<N_PARAMS>,
        // TODO: can we remove this 'static bound?
        Ret: Serialize + 'static,
    {
        let name: Arc<str> = Arc::from(name.to_string());
        let callable: Arc<AnyCallable<SelectContext, Ret, std::convert::Infallible>> =
            Arc::from(handler.into_callable(()));

        let dependencies = Params::get_select_dependencies();

        for dependency in &dependencies {
            if let SelectDependencyType::Store(type_id) = dependency {
                self.select_dependencies
                    .entry(*type_id)
                    .or_default()
                    .insert(name.clone());
            }
        }

        self.selects.insert(
            name.clone(),
            AnySelect {
                name,
                select: Arc::new(move |ctx| {
                    let callable = callable.clone();
                    Box::pin(async move {
                        let result = callable(ctx).await.expect("Select should not fail");
                        serde_json::to_value(result)
                    })
                }),
                depends_on_stores: dependencies
                    .iter()
                    .filter_map(|d| {
                        if let SelectDependencyType::Store(type_id) = d {
                            Some(*type_id)
                        } else {
                            None
                        }
                    })
                    .collect(),
            },
        );

        self
    }

    /// Declare a store
    pub fn state<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        self.states.insert(state);
        self
    }

    /// Declare a hook function. TODO: write documentation for this.
    pub fn hook<Params, Return, Handler, const IS_ASYNC: bool>(
        mut self,
        method: impl ToString,
        handler: Handler,
    ) -> Self
    where
        Handler: IntoCallable<HookContext, Params, Return, HookError, HookInit, IS_ASYNC>,
        Return: Serialize + 'static,
    {
        let method = method.to_string();

        let callable: Arc<AnyCallable<HookContext, Return, HookError>> =
            Arc::from(handler.into_callable(HookInit {}));

        self.hooks.add_hook_function(HookFunction {
            type_id: std::any::TypeId::of::<Handler>(),
            method: method.clone(),
            callable: Box::new(move |ctx| {
                let callable = callable.clone();

                Box::pin(async move {
                    let result = callable(ctx).await?;

                    Ok(HookResponse {
                        body: bindgen::HookBody::Json(serde_json::to_string(&result)?),
                    })
                })
            }),
        });

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
            states: self.states,
            rpc_functions: self.rpc_functions,
            on_connect: self.on_connect,
            on_disconnect: self.on_disconnect,
            background: self.background,
            hooks: self.hooks,
            selects: self.selects,
            select_dependencies: self.select_dependencies,
        };

        let app = App {
            inner: Arc::new(inner),
        };

        #[cfg(feature = "typed")]
        app.export_types();

        app
    }
}

#[macro_export]
macro_rules! register {
    ($func:ident) => {
        pub use $crate::bindings::bindgen::{
            self, __export_world_imports_cabi, _export_dry_run_cabi, _export_run_cabi, export,
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

            fn dry_run() -> Result<(), ()> {
                $crate::bindings::init_panic_hook();
                $crate::tasks::Runtime::new().global();
                let _app = $func();
                Ok(())
            }
        }

        #[allow(unsafe_op_in_unsafe_fn)]
        export!(GuestImpl);
    };
}
