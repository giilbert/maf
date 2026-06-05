//! Utilities for defining MAF applications.
//!
//! An application is a collection of functionality such as stores, RPC functions, and
//! background tasks. The [`App`] struct contains the configuration for the application, and is
//! built using the [`AppBuilder`].
//!
//! ## Example
//! ```rust,no_run
//! use maf::prelude::*;
//!
//! // ... application functionality goes here ...
//!
//! fn build() -> App {
//!     // Begin with a call to App::builder()
//!     App::builder()
//!         // Add stores, RPC functions, background tasks, etc.
//!         // .store::<Message>()
//!         // .rpc("set_message", set_message)
//!         // .background(cleanup_task)
//!         // .plugin(MyPlugin::new())
//!         // Finish building the app
//!         .build()
//! }
//!
//! // IMPORTANT: register the function that builds the app when using WASM
//! maf::register!(build);
//! ```
//!
//! ## APIs
//! [`AppBuilder`] provides methods to add functionality to the application, such as:
//! - Adding [`crate::Store`] with [`AppBuilder::store`]
//! - Registering [`crate::rpc`] functions with [`AppBuilder::rpc`]
//! - Adding background tasks with [`AppBuilder::background`]
//! - Registering plugins with [`AppBuilder::plugin`]
//! - Defining local state with [`AppBuilder::local`]
//! - Subscribing meta entries with [`AppBuilder::meta`]

mod background;
pub(crate) mod hooks;
mod impls;
mod local;
pub(crate) mod meta;
pub(crate) mod observe;
mod on_connect_disconnect;
pub(crate) mod plugin;

pub(crate) use impls::AppState;
pub use impls::{App, AppBuilder};
pub use local::Local;
pub(crate) use local::LocalStateError;
pub use maf_schemas::apps::{MetaEntry, MetaVisibility};
pub use plugin::Plugin;
