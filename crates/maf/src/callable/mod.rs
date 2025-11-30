//! Core functionality for writing and managing variadic callback functions like `.on_connect(...)`
//! and RPC methods.
//!
//! For a high-level overview, the [`IntoCallable`] trait is the main item of concern here. It has
//! 6 type parameters: `<Ctx, Params, Ret, Err, Init: Send, const IS_ASYNC: bool>`, each describing
//! a different aspect of a the callback. The type parameters are used when registering/storing the
//! callable to allow different types of callables to be registered using the same method.
//!
//! - `Ctx`: The context type passed to the callable when it is invoked. This contains information
//!   about the environment in which the callable is being executed. For example, in RPC methods,
//!   [`crate::rpc::RpcRequestContext`] contains information about the RPC request being handled
//!   (e.g. the user making the request, the method being called, parameters, etc.).
//! - `Params`: A tuple of parameter types that the callable function accepts. Each type in the
//!   tuple must implement the [`CallableParam`] trait for the given `Ctx` and `Init` types.
//! - `Ret`: The return type of the callable function, excluding the `impl Future<Output = ...>`
//!   wrapper for async callables.
//! - `Err`: The error type that the callable function may return. This is regarding preparation and
//!   execution of the callable, not errors that may occur within the callable itself.
//! - `Init`: An initialization type that is used to provide additional context or configuration
//!   when extracting parameters for the callable.
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
//! pub fn rpc<
//!     Params,
//!     Return,
//!     const IS_ASYNC: bool,
//!     // Varies by function signature          vvvvvv  vvvvvv                            vvvvvvvv
//!     Handler: IntoCallable<RpcRequestContext, Params, Return, RpcError, RpcRequestInit, IS_ASYNC>,
//!     // Specific to RPCs   ^^^^^^^^^^^^^^^^^                  ^^^^^^^^  ^^^^^^^^^^^^^^
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
