use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use maf_schemas::packet::{
    Bull, ChannelSendRx, OneStoreUpdate, RxPacket, TxPacket, TypedRpcRequestPacket,
    TypedRpcResponsePacket,
};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{
    mpsc::{self, error::TryRecvError},
    RwLock,
};
use uuid::Uuid;

use crate::{
    app::{
        background::BackgroundFnContext,
        hooks::{HookBody, HookRequest, HookResponse},
        meta::{AnyMetaUpdater, MetaContext, MetaKey, MetaStorage},
        observe::{ObserveDepdendency, ObserveStore, ObserveTarget},
    },
    callable::{BoxedCallable, IntoCallable},
    channel::UntypedChannelBroadcast,
    platform::{ListenError, Platform, TargetPlatform},
    rpc::{RpcError, RpcRequestContext, RpcRequestInit, RpcStore},
    store::{
        AnySelect, AnyStore, GetParamSelectDependencies, SelectContext, SelectDependencyType,
        SelectKey, StoreId,
    },
    tasks::{self},
    user::{UserMessage, UserNextMessageError},
    Channel, Local, MetaVisibility, RpcFunction, Store, StoreData, User,
};

use super::{
    background::{BackgroundFn, BackgroundFnError},
    hooks::{HookContext, HookError, HookFunction, HookStore},
    local::LocalStateStore,
    on_connect_disconnect::{
        OnConnectDiconnectContext, OnConnectDisconnectError, OnConnectDisconnectFn,
    },
};

/// A complete MAF application, containing stores, RPC functions, background tasks, and more.
#[derive(Clone)]
pub struct App {
    pub(crate) inner: Arc<AppInner>,
}

// TODO: REFACTOR ME!!!
pub(crate) struct AppInner {
    pub(crate) state: Arc<AppState>,
    pub(crate) rpc_functions: RpcStore,
    pub(crate) states: LocalStateStore,
    pub(crate) hooks: HookStore,
    pub(crate) store_dirty_rx: RwLock<mpsc::Receiver<StoreId>>,
    pub(crate) on_connect: Option<Arc<OnConnectDisconnectFn>>,
    pub(crate) on_disconnect: Option<Arc<OnConnectDisconnectFn>>,
    pub(crate) background: Option<Arc<BackgroundFn>>,
    pub(crate) selects: HashMap<SelectKey, AnySelect>,
    pub(crate) platform: Arc<TargetPlatform>,
    pub(crate) observe: ObserveStore,
    pub(crate) meta: MetaStorage,
}

#[derive(Debug)]
pub struct AppState {
    pub(crate) users: RwLock<HashMap<Uuid, User>>,
    pub(crate) stores: RwLock<HashMap<StoreId, AnyStore>>,
    pub(crate) store_dirty: mpsc::Sender<StoreId>,
    pub(crate) channels: RwLock<HashMap<String, UntypedChannelBroadcast>>,
    pub(crate) user_rx_channels: RwLock<HashMap<(Uuid, String), UntypedChannelBroadcast>>,
}

// FIXME: #[derive(Default)] makes it so that structs stored in the builder must also implement
// Default. This is not ideal because it allows some structs to be constructed when they should not
// be.
/// Builder for constructing a MAF application. Used to register stores, RPC functions, background
/// tasks, and more.
#[derive(Default)]
pub struct AppBuilder {
    on_connect: Option<Arc<OnConnectDisconnectFn>>,
    on_disconnect: Option<Arc<OnConnectDisconnectFn>>,
    background: Option<Arc<BackgroundFn>>,
    rpc_functions: RpcStore,
    local_states: LocalStateStore,
    hooks: HookStore,
    stores: HashMap<StoreId, AnyStore>,
    selects: HashMap<SelectKey, AnySelect>,
    observe: ObserveStore,
    meta: MetaStorage,
    platform: Option<Arc<TargetPlatform>>,
}

// TODO: Link documentation
impl App {
    /// Begin building a new application.
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }

    /// Fetches a store instance for the given store data type. If the store does not exist, it
    /// will be created and initialized with the default value.
    pub async fn store<T: StoreData>(&self) -> Store<T> {
        Store::<T>::new(self.clone()).await
    }

    /// Fetches a local state instance for the given type.
    pub async fn local<T: Send + Sync + 'static>(&self) -> Local<T> {
        self.inner
            .states
            .get::<T>()
            .expect("local state does not exist")
    }

    /// Access the meta storage associated with this app.
    pub fn meta(&self) -> &MetaStorage {
        &self.inner.meta
    }

    async fn handle_connections(self) -> anyhow::Result<()> {
        // TODO: handle errors
        loop {
            let user = User::new(
                self.inner.state.clone(),
                self.inner.platform.next_user().await?,
            );

            self.inner
                .state
                .users
                .write()
                .await
                .insert(user.meta().id, user.clone());

            let app = self.clone();
            tasks::spawn(async move { app.trigger_update(&ObserveDepdendency::Users).await });

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

                loop {
                    let message = match user_clone.next_message().await {
                        Ok(message) => message,
                        Err(UserNextMessageError::Listen(ListenError::Closed)) => {
                            // User has disconnected
                            break;
                        }
                        Err(e) => {
                            println!("failed to get next message: {e}");
                            continue;
                        }
                    };

                    if let Err(e) = app.handle_message(message).await {
                        println!("failed to handle message: {e}");
                    }
                }

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
                    .remove(&user_clone.meta().id);

                app.trigger_update(&ObserveDepdendency::Users).await.ok();
            });
        }
    }

    async fn handle_hook_requests(self) -> anyhow::Result<()> {
        loop {
            let request_raw = self.inner.platform.next_hook_request().await?;
            let request = HookRequest::new(self.inner.state.clone(), request_raw)?;

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
            .get(&(user.meta().id, channel_data.channel.clone()))
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

    pub(super) async fn compute_select_contents(
        &self,
        name: &SelectKey,
        user: User,
    ) -> anyhow::Result<Value> {
        let any_select = self
            .inner
            .selects
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("select not found: {name:?}"))?;

        let value = (any_select.select)(SelectContext {
            app: self.clone(),
            user,
        })
        .await
        .map_err(|e| anyhow::anyhow!("failed to compute select contents for {name:?}: {e}"))?;

        Ok(value)
    }

    /// TODO: This api shouldnt be public
    /// FIXME: This is a temporary workaround to flush store changes where change detection does not
    /// work, such as in .background tasks
    pub async fn flush_all_store_changes(&self) -> anyhow::Result<()> {
        let mut store_dirty_rx = self.inner.store_dirty_rx.write().await;

        loop {
            let store_id = match store_dirty_rx.try_recv() {
                Ok(store_id) => store_id,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    anyhow::bail!("store dirty channel disconnected");
                }
            };

            self.flush_store_change(&store_id).await?;
        }

        Ok(())
    }

    pub(crate) async fn get_any_store(&self, id: &StoreId) -> anyhow::Result<AnyStore> {
        let stores = self.inner.state.stores.read().await;

        let store = stores
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("store not found: {:?}", id))?;

        Ok(store)
    }

    pub(crate) async fn serialize_store(
        &self,
        user: User,
        store: AnyStore,
    ) -> anyhow::Result<Value> {
        let data = store.data.read_owned().await;
        let serialized_data = (store.serializer)(&*data, &user)?;

        Ok(serialized_data)
    }

    /// TODO: See comment above
    pub(crate) async fn flush_store_change(&self, id: &StoreId) -> anyhow::Result<()> {
        self.trigger_update(&ObserveDepdendency::Store(*id)).await?;
        Ok(())
    }

    // TODO: allow users to subscribe to stores instead of sending updates optimistically
    async fn refresh_all_stores(&self, user: &User) -> anyhow::Result<()> {
        let stores = self.inner.state.stores.read().await;

        let mut data: Vec<(&str, serde_json::Value)> = Vec::with_capacity(stores.len());

        for (_store_id, store) in stores.iter() {
            let serialized = (store.serializer)(&*store.data.read().await, user)
                .context("failed to serialize store")?;

            data.push((&store.name, serialized));
        }

        // Also compute every select
        for (select_name, ..) in self.inner.selects.iter() {
            let value = self
                .compute_select_contents(select_name, user.clone())
                .await?;

            data.push((&select_name.0, value));
        }

        user.send(TxPacket::ManyStoreUpdate::<serde_json::Value>(
            data.iter()
                .map(|(k, v)| OneStoreUpdate {
                    store: k.as_ref(),
                    data: Bull::Borrowed(v),
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
        #[cfg(not(feature = "native"))]
        tasks::Runtime::current().blocking_poll();
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
    /// use maf::prelude::*;
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
    /// use maf::prelude::*;
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
    /// use maf::prelude::*;
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
        #[cfg(feature = "typed")] Handler: IntoCallable<RpcRequestContext, Params, Return, RpcError, RpcRequestInit, IS_ASYNC>
            + crate::typed::ExtractRpcDesc<TypedParams, TypedReturn, TYPED_IS_ASYNC>,
        #[cfg(not(feature = "typed"))] Handler: IntoCallable<RpcRequestContext, Params, Return, RpcError, RpcRequestInit, IS_ASYNC>,
    >(
        mut self,
        method: impl ToString,
        handler: Handler,
    ) -> Self
    where
        Return: Serialize + 'static,
    {
        use std::any::Any;

        let method = method.to_string();
        let callable: Arc<BoxedCallable<RpcRequestContext, Return, RpcError>> =
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
            desc: Arc::new(move |generator| Handler::extract(generator, method.clone())),
        });
        self
    }

    /// Register a task to run in the background.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use maf::prelude::*;
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
        self.stores.insert(StoreId::of::<T>(), AnyStore::new::<T>());
        self
    }

    /// Register a store where its contents are derived with the provided function.
    pub fn select<
        Name: ToString,
        Params,
        Ret,
        #[cfg(not(feature = "typed"))] Handler: IntoCallable<SelectContext, Params, Ret, std::convert::Infallible, (), IS_ASYNC>,
        #[cfg(feature = "typed")] Handler: IntoCallable<SelectContext, Params, Ret, std::convert::Infallible, (), IS_ASYNC>
            + crate::typed::ExtractSelectDesc<Params, Ret, IS_ASYNC>,
        const IS_ASYNC: bool,
        const N_PARAMS: usize,
    >(
        mut self,
        name: Name,
        handler: Handler,
    ) -> Self
    where
        Params: GetParamSelectDependencies<N_PARAMS>,
        // TODO: can we remove this 'static bound?
        Ret: Serialize + 'static,
    {
        let name = SelectKey(Arc::from(name.to_string()));
        let callable = Arc::new(handler.into_callable(()));

        for dependency in Params::get_select_dependencies() {
            self.observe.add_dependency(
                match dependency {
                    SelectDependencyType::Store(store_id) => ObserveDepdendency::Store(store_id),
                    SelectDependencyType::Users => ObserveDepdendency::Users,
                    SelectDependencyType::None => continue,
                },
                ObserveTarget::Select(name.clone()),
            );
        }

        self.selects.insert(
            name.clone(),
            AnySelect {
                name: name.clone(),
                select: Box::new(move |ctx| {
                    let callable = callable.clone();
                    Box::pin(async move {
                        let result = callable(ctx).await.expect("Select should not fail");
                        serde_json::to_value(result)
                    })
                }),
                #[cfg(feature = "typed")]
                desc: Arc::new(move |generator| Handler::extract(generator, name.0.to_string())),
            },
        );

        self
    }

    /// Subscribes a meta entry to be automatically updated when its dependencies change.
    pub fn meta<
        Name: ToString,
        Params,
        Ret,
        Handler: IntoCallable<MetaContext, Params, Ret, std::convert::Infallible, (), IS_ASYNC>,
        const IS_ASYNC: bool,
        const N_PARAMS: usize,
    >(
        mut self,
        visibility: MetaVisibility,
        name: Name,
        handler: Handler,
    ) -> Self
    where
        // TODO: This concept of a "dependency" can be generalized for other use cases. It is fine
        // for now since meta and select have the same types of dependencies.
        Params: GetParamSelectDependencies<N_PARAMS>,
        // TODO: can we remove this 'static bound?
        Ret: Serialize + 'static,
    {
        let key = MetaKey(name.to_string().into());
        let handler = Arc::new(handler.into_callable(()));

        for dependency in Params::get_select_dependencies() {
            self.observe.add_dependency(
                match dependency {
                    SelectDependencyType::Store(store_id) => ObserveDepdendency::Store(store_id),
                    SelectDependencyType::Users => ObserveDepdendency::Users,
                    SelectDependencyType::None => continue,
                },
                ObserveTarget::Meta(key.clone()),
            );
        }

        self.meta.updaters.insert(
            key.clone(),
            AnyMetaUpdater {
                _key: key,
                visibility,
                create: Box::new(move |ctx| {
                    let handler = handler.clone();
                    Box::pin(async move {
                        let result = handler(ctx).await.expect("infallible");
                        serde_json::to_value(result)
                    })
                }),
            },
        );

        self
    }

    /// Declares a [`crate::Local`], a piece of state that **does not need to be synchronized** with
    /// connect clients. If synchronization with clients is needed, use [`crate::Store`]
    ///
    /// The initial value of the local state should be provided as an argument.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use maf::prelude::*;
    ///
    /// struct Points {
    ///     points: u32,
    /// }
    ///
    /// // [`Points`] does not need to be `serde::Serialize` and is not visible to clients
    /// async fn score_point(points: Local<Points>) {
    ///     let points = points.write().await;
    ///     points += 1;
    ///     if points > 100 {
    ///         println!("You win!");
    ///     }
    /// }
    ///
    /// fn build() -> App {
    ///     App::builder()
    ///         .local(Points { points: 0 })
    ///         .rpc("score_point", score_point)
    ///         .build()
    /// }
    /// ```
    pub fn local<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        self.local_states.insert(state);
        self
    }

    /// Declare a hook function. TODO: write documentation for this.
    pub fn hook<Params, Return, Handler, const IS_ASYNC: bool>(
        mut self,
        method: impl ToString,
        handler: Handler,
    ) -> Self
    where
        Handler: IntoCallable<HookContext, Params, Return, HookError, (), IS_ASYNC>,
        Return: Serialize + 'static,
    {
        let method = method.to_string();

        let callable: Arc<BoxedCallable<HookContext, Return, HookError>> =
            Arc::from(handler.into_callable(()));

        self.hooks.add_hook_function(HookFunction {
            type_id: std::any::TypeId::of::<Handler>(),
            method: method.clone(),
            callable: Box::new(move |ctx| {
                let callable = callable.clone();

                Box::pin(async move {
                    let result = callable(ctx).await?;

                    Ok(HookResponse {
                        body: HookBody::Json(serde_json::to_string(&result)?),
                    })
                })
            }),
        });

        self
    }

    /// Binds the MAF app to a specified [`TargetPlatform`].
    ///
    /// See [`crate::platform`] for more details on platforms.
    pub fn platform(mut self, platform: TargetPlatform) -> Self {
        self.platform = Some(Arc::new(platform));
        self
    }

    pub fn build(mut self) -> App {
        const STORE_UPDATE_LIMIT: usize = 10_000;

        let (store_dirty, store_dirty_rx) = mpsc::channel(STORE_UPDATE_LIMIT);

        let state = Arc::new(AppState {
            store_dirty,
            channels: Default::default(),
            stores: RwLock::new(self.stores),
            user_rx_channels: Default::default(),
            users: Default::default(),
        });

        let platform = self.platform.unwrap_or_else(|| {
            Arc::new(TargetPlatform::init(()).expect("Failed to initialize platform"))
        });
        self.meta.platform = Some(platform.clone());

        let inner = AppInner {
            state,
            store_dirty_rx: RwLock::new(store_dirty_rx),
            states: self.local_states,
            rpc_functions: self.rpc_functions,
            on_connect: self.on_connect,
            on_disconnect: self.on_disconnect,
            background: self.background,
            hooks: self.hooks,
            selects: self.selects,
            observe: self.observe,
            meta: self.meta,
            platform,
        };

        let app = App {
            inner: Arc::new(inner),
        };

        #[cfg(feature = "typed")]
        app.export_types();

        app
    }
}

#[cfg(not(feature = "native"))]
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
