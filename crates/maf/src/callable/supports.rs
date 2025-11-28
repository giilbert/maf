//! Conditional support for various callable types.

/// A marker trait for [`crate::callable::CallableParam`]'s that support being ran in an async
/// context.
pub trait SupportsAsync {}
