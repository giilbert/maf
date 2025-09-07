//! Platform-specific implementations for MAF.

#[cfg(feature = "native")]
mod actor;
mod common;
mod types;
#[cfg(not(feature = "native"))]
mod wasi;

pub use common::*;
pub use types::*;

#[cfg(feature = "native")]
pub use actor::{ActorPlatform as TargetPlatform, ActorPlatformHandle, RawHookRequest, RawUser};
#[cfg(not(feature = "native"))]
pub use wasi::{RawHookRequest, RawUser, WasiPlatform as TargetPlatform};
