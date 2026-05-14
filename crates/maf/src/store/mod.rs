//! Abstraction for state shared between server and client.

mod impls;
mod pointers;
mod select;

pub(crate) use impls::*;
pub use impls::{Store, StoreData};
pub use pointers::{StoreMut, StoreRef};
pub(crate) use select::*;
