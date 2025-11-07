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

mod app;
mod background;
pub(crate) mod hooks;
mod on_connect_disconnect;
mod plugin;
mod state;

pub use app::{App, AppBuilder};
pub use plugin::Plugin;
pub use state::State;

pub(crate) use app::AppState;
pub(crate) use state::StateError;
