//! Abstraction for state shared between server and client.

mod change_detection;
mod select;
mod store;

pub(crate) use select::*;
pub(crate) use store::*;
pub use store::{Store, StoreData};
