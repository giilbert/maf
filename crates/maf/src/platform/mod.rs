//! Platform-specific implementations for MAF.
//!
//! This crate contains code that enables MAF applications to run on WASI standards (`wasi` module)
//! and native code (`native` module). By default, MAF is set to compile for WASI. The `native`
//! feature can be enabled to compile to native code.

#[cfg(feature = "native")]
mod actor;
mod common;
mod types;
#[cfg(not(feature = "native"))]
mod wasi;

#[cfg(feature = "native")]
pub use actor::{ActorPlatform as TargetPlatform, ActorPlatformHandle, RawHookRequest, RawUser};
pub use common::*;
pub use types::*;
#[cfg(not(feature = "native"))]
pub use wasi::{RawHookRequest, RawUser, WasiPlatform as TargetPlatform};
