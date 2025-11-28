//! Core functionality for writing and managing variadic callback functions like `.on_connect(...)`
//! and RPC methods.
//!
//! For a high-level overview, the [`IntoCallable`] trait is the main item of concern here. It has
//! 6 type parameters: `<Ctx, Params, Ret, Err, Init: Send, const IS_ASYNC: bool>`, each describing
//! a different aspect of a the callback. The type parameters are used when registering/storing the
//! callable to allow different types of callables to be registered using the same method.
//!
//! [`IntoCallable`] is implemented for functions that take any number of parameters (up to 8)
//! which all implement the [`CallableParam`] trait for the given `Ctx` and `Init` types. For more
//! information on what implements [`CallableParam`] for what `Ctx` and `Init`, see the docs for
//! individual implementations of callable functions.
//!
//! ## Example
//! An example of this in action is in [`crate::rpc`] (the RPC functionality of MAF).
//!
//! 1. RPCs are registered (declared) in [`crate::AppBuilder::rpc`] like:
//! ```rust
//!
//! pub fn rpc<
//!     Params,
//!     Return,
//!     const IS_ASYNC: bool,
//!     Handler: IntoCallable<RpcRequestContext, Params, Return, RpcError, RpcRequestInit, IS_ASYNC>,
//! >(
//!     mut self,
//!     method: impl ToString,
//!     handler: Handler,
//! ) -> Self
//! where
//!     Return: Serialize + 'static,
//! {
//!     // ...
//! }
//! ```
//!
//! 2. Where it is converted to a common type with:
//! ```rust
//! handler.into_callable(RpcRequestInit {
//!     method: method.clone(),
//! });
//! ```
//!
//! 3. And finally invoked with:
//! ```rust
//! let ctx = RpcRequestContext { ... };
//! let result = callable(ctx).await?;
//! ```
//!
//! Please forgive me.

mod callable;
mod params;
mod supports;

pub use callable::*;
pub use params::*;
pub use supports::*;
