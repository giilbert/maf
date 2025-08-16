//! This module provides core functionality for writing and managing variadic callback functions
//! like `.on_connect(...)` and RPC methods.
//!
//! For a high-level overview, the [`IntoCallable`] trait is the main item of concern here. It has
//! 6 type parameters: `<Ctx, Params, Ret, Err, Init: Send, const IS_ASYNC: bool>`, each describing
//! a different aspect of a the callback. The type parameters are used when registering/storing the
//! callable to allow different types of callables to be registered using the same method.
//!
//! Please forgive me.

mod callable;
mod params;

pub use callable::*;
pub use params::*;
