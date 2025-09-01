//! Contains shared schemas used in MAF crates.
//!
//! This crate is not intended to be used directly by end users, but rather as a dependency for
//! writing other MAF crates:
//!
//! - [`apps`]: describing MAF apps on Platform.
//! - [`error`]: API error responses. A `error-response` feature flag is needed to
//!   enable and compile this module.
//! - [`packet`]: packets sent over WebSocket between clients and servers.
//! - [`project_config`]: the `maf-project.toml` project configuration file.
//! - [`typed`]: types that describe a MAF app's type data

pub mod apps;
pub mod packet;
pub mod project_config;
pub mod typed;

#[cfg(feature = "error-response")]
pub mod error;
