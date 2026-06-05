#![cfg_attr(docsrs, feature(doc_cfg))]

//! **Mutation Authority Framework (MAF) is an authoritative realtime framework for writing simple,
//! secure, and scalable apps.**
//!
//! Instead of writing complex route handlers, state synchronization logic, and security checks,
//! you can focus on writing the core logic of your app while MAF handles the rest. Here's an
//! example app that uses MAF to synchronize a button's state across multiple clients with just a
//! few lines of code can be:
//!
//! ```rust,no_run
//! use maf::prelude::*;
//!
//! // A store holds state shared between client and server.
//! // Here we define a simple counter store.
//! struct Counter {
//!     count: i32,
//! }
//!
//! // Behavior of the Counter store is implemented by implementing the StoreData trait.
//! impl StoreData for Counter {
//!     type Select<'this> = i32;
//!
//!     fn init() -> Self {
//!         Counter { count: 0 }
//!     }
//!
//!     fn select(&self, _user: &User) -> Self::Select<'_> {
//!         self.count
//!     }
//!
//!     fn name() -> impl AsRef<str> {
//!         "count" // Used by the frontend to identify the store
//!     }
//! }
//!
//! // An RPC function that increments the counter by a given amount.
//! // MAF is authoritative in that all state changes must go through the server,
//! // so clients cannot modify the counter directly.
//! async fn add_counter(Params(amount): Params<i32>, counter: Store<Counter>) -> i32 {
//!     let mut store = counter.write().await;
//!     store.count += amount;
//!     store.count
//! }
//!
//! // Build the MAF app, registering the counter store and the RPC function.
//! fn build() -> App {
//!     App::builder()
//!         .store::<Counter>()
//!         .rpc("add_counter", add_counter)
//!         .build()
//! }
//!
//! maf::register!(build);
//! ```
//!
//! ## Getting Started
//! Check out the [Quick Start Guide](https://maf.gilbertz.me/docs/getting-started/quickstart) on
//! the [website](https://maf.gilbertz.me) for the easiest way to get started launching with MAF.
//! The quick start guide will walk you through setting up a new MAF project, running a local
//! development server, and deploying your app on MAF Platform. This website (docs.rs) serves as the
//! API reference for MAF's Rust crates.
//!
//! ⚠️ NOTE: MAF is still in very early development: some things are not easy to do, APIs and
//! features are subject to change, and things will break!

pub mod app;
pub mod callable;
pub mod channel;
#[cfg(any(doc, not(feature = "native")))]
pub mod http;
pub mod platform;
pub mod rpc;
pub mod store;
pub mod tasks;
pub mod user;

#[doc(hidden)]
pub mod bindings;
#[cfg(feature = "typed")]
pub mod typed;

pub(crate) use app::LocalStateError;

/// Re-exports of commonly used items from MAF and its dependencies.
pub mod prelude {
    pub use ::serde;
    pub use ::serde_json;
    pub use ::wasi;
    pub use app::{App, AppBuilder, Local, MetaEntry, MetaVisibility, Plugin};
    pub use channel::{Channel, RecvError};
    pub use rpc::{Params, RawRpcRequest, RpcFunction};
    pub use store::{Store, StoreData, StoreMut, StoreRef};
    pub use user::{User, Users};
    pub use uuid::Uuid;

    use super::*;
}

// For usage inside MAF, prelude is always available.
pub(crate) use prelude::*;
