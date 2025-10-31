//! Contains shared schemas used in Cobble crates.
//!
//! This crate is not intended to be used directly by end users, but rather as a dependency for
//! writing other Cobble crates:
//!
//! - [`apps`]: describing Cobble apps on Platform.
//! - [`packet`]: packets sent over WebSocket between clients and servers.
//! - [`project_config`]: the `cobble-project.toml` project configuration file.
//! - [`typed`]: types that describe a Cobble app's type data
//!
//! - (enabled with the `error-response` feature) `error`: API error responses.

pub mod apps;
pub mod packet;
pub mod project_config;
pub mod typed;

#[cfg(feature = "error-response")]
pub mod error;
