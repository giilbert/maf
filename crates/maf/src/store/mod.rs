//! Abstraction for state shared between server and client.

mod pointers;
mod select;
mod store;

pub(crate) use select::*;
pub(crate) use store::*;
pub use store::{Store, StoreData};
