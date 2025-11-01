//! Cobble is an authoritative realtime framework for writing simple, secure, and scalable apps.
//!
//! Check out the [Quick Start Guide](https://cobble.gilbertz.me/docs/getting-started/quickstart) on
//! the [website](https://cobble.gilbertz.me) for the easiest way to get started launching with Cobble.
//!
//! ⚠️ NOTE: Cobble is still in very early development: some things are not easy to do, APIs and
//! features are subject to change, and things will break!

pub mod app;
pub mod bindings;
pub mod callable;
pub mod channel;
#[cfg(not(feature = "native"))]
pub mod http;
pub mod platform;
pub mod rpc;
pub mod store;
pub mod tasks;
pub mod user;

#[cfg(feature = "typed")]
pub mod typed;

pub(crate) use app::StateError;

/// Re-exports of commonly used items from Cobble and its dependencies.
pub mod prelude {
    use super::*;

    pub use app::{App, AppBuilder, Plugin, State};
    pub use channel::{Channel, RecvError};
    pub use rpc::{Params, RpcFunction, RpcRequest};
    pub use store::{Store, StoreData};
    pub use user::User;
    pub use uuid::Uuid;

    pub use serde;
    pub use serde_json;
    pub use wasi;
}

// For usage inside Cobble, prelude is always available.
pub(crate) use prelude::*;
