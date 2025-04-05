use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc, sync::Arc};

use serde_json::json;

use crate::{
    rpc::{IntoRpcFunction, RpcStore},
    tasks::{self, Runtime},
    User, UserListener,
};

#[doc(hidden)]
pub static GLOBAL_APP: GlobalApp = GlobalApp::new();

#[repr(transparent)]
pub struct GlobalApp(RefCell<Option<Rc<App>>>);

unsafe impl Sync for GlobalApp {}

impl GlobalApp {
    const fn new() -> Self {
        Self(RefCell::new(None))
    }

    pub fn get(&self) -> Rc<App> {
        self.0
            .borrow()
            .as_ref()
            .expect("global app not registered")
            .clone()
    }

    pub fn register(&self, app: App) {
        assert!(
            cfg!(target_arch = "wasm32"),
            "global app can only be used in WebAssembly"
        );

        if self.0.borrow().is_some() {
            panic!("global app already registered");
        }
        *self.0.borrow_mut() = Some(Rc::new(app));
    }
}

type OnConnectFn = dyn Fn(User) -> Pin<Box<dyn Future<Output = ()>>> + Send;
pub struct App {
    pub(crate) rpc_functions: RpcStore,
    pub(crate) on_connect: Option<Arc<OnConnectFn>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            rpc_functions: RpcStore::default(),
            on_connect: None,
        }
    }

    pub fn on_connect<T>(mut self, handler: impl IntoOnConnect<T>) -> Self {
        self.on_connect = Some(handler.into_on_connect());
        self
    }

    pub fn add_rpc_function<R, P>(
        mut self,
        path: impl ToString,
        handler: impl IntoRpcFunction<R, P>,
    ) -> Self {
        let path = path.to_string();
        self.rpc_functions
            .add_rpc_function(handler.into_rpc_function(path));
        self
    }

    async fn handle_connections(self: Arc<Self>) -> anyhow::Result<()> {
        let users = UserListener::new()?;

        loop {
            let user = users.next().await?;

            self.on_connect.as_ref().map(|handler| {
                let handler = handler.clone();
                tasks::spawn(handler(user));
            });
        }
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
}

#[macro_export]
macro_rules! register_build {
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

// An R type parameter is needed to allow for different types of return values
pub trait IntoOnConnect<R> {
    fn into_on_connect(self) -> Arc<OnConnectFn>;
}

impl<F: Fn(User) -> () + Clone + Send + Sync + 'static> IntoOnConnect<()> for F {
    fn into_on_connect(self) -> Arc<OnConnectFn> {
        Arc::new(move |user| {
            let f = self.clone();
            Box::pin(async move {
                f(user);
            })
        })
    }
}

impl<F: Fn(User) -> R + Clone + Send + Sync + 'static, R: Future<Output = ()>>
    IntoOnConnect<Pin<Box<dyn Future<Output = ()>>>> for F
{
    fn into_on_connect(self) -> Arc<OnConnectFn> {
        Arc::new(move |user| {
            let f = self.clone();
            Box::pin(async move {
                f(user).await;
            })
        })
    }
}
