use std::{
    cell::{RefCell, RefMut, UnsafeCell},
    collections::HashMap,
    rc::Rc,
    sync::{RwLock, RwLockWriteGuard},
};

use crate::rpc::{IntoRpcFunction, RpcStore};

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

pub struct App {
    pub(crate) rpc_functions: RpcStore,
}

impl App {
    pub fn new() -> Self {
        Self {
            rpc_functions: RpcStore::default(),
        }
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
}

#[macro_export]
macro_rules! register_build {
    ($func:ident) => {
        #[no_mangle]
        pub extern "C" fn init() {
            $crate::bindings::init_panic_handler();
            let app = $func();
            $crate::app::GLOBAL_APP.register(app);
        }
    };
}
