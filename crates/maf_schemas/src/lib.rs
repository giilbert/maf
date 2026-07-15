//! Contains shared schemas used in MAF crates.
//!
//! This crate is not intended to be used directly by end users, but rather as a dependency for
//! writing other MAF crates:
//!
//! - [`apps`]: describing MAF apps on Platform.
//! - [`packet`]: packets sent over WebSocket between clients and servers.
//! - [`project_config`]: the `maf-project.toml` project configuration file.
//! - [`typed`]: types that describe a MAF app's type data
//! - [`admin`]: types used in the admin API.
//!
//! - (enabled with the `error-response` feature) `error`: API error responses.

pub mod admin;
pub mod apps;
pub mod packet;
pub mod project_config;
pub mod typed;

#[cfg(feature = "error-response")]
pub mod error;
#[cfg(feature = "error-response")]
pub use error::ErrorResponse;
