//! Abstraction for state shared between server and client.

mod pointers;
mod select;
mod store;

pub use pointers::{StoreMut, StoreRef};
pub(crate) use select::*;
pub(crate) use store::*;
pub use store::{Store, StoreData};
