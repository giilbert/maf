//! Shared interfaces used by different platforms. Currently, this is only used for WASI and native
//! platforms, but it is designed to be extensible for other platforms in the future.
//!
//! #[cfg(..)] attributes are used to run code conditionally based on the target platform. e.g. no
//! trait objects are instantiated on runtime: there are no `Box<dyn Platform>`'s or
//! `impl Platform`'s anywhere. This is to ensure simplicity and avoid dynamic dispatch overhead.

use std::future::Future;

use crate::{
    app::hooks,
    platform::{self, ListenError, SendError},
    user::{UserMeta, UserNextMessageError},
};

pub trait Platform {
    type Config;

    fn init(config: Self::Config) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Returns a future that resolves to the next user that has connected to the app.
    fn next_user(&self) -> impl Future<Output = Result<platform::RawUser, ListenError>>;

    /// Returns a future that resolves to the next hook request requested by the host.
    fn next_hook_request(
        &self,
    ) -> impl Future<Output = Result<platform::RawHookRequest, ListenError>>;

    fn report_app_schema(&self, schema: &str);
}

pub trait PlatformUser {
    /// Retrieves the metadata of the user, such as their ID and authentication payload.
    fn meta(&self) -> UserMeta;

    /// Asynchronously sends a message to the user. There is no guarantee that the message is
    /// processed immediately or at all or that the message is received in the order they were
    /// sent.
    ///
    /// If the user has disconnected, this method should return `Err(SendError::Closed)`.
    fn send(&self, message: platform::Message) -> Result<(), SendError>;

    /// Returns a future that resolves to the next message sent by the user. The implementation
    /// should queue messages and return them in the order they were sent.
    ///
    /// If the user has disconnected, the future should resolve with
    /// `Err(UserNextMessageError::Listen(ListenError::Closed))`.
    fn next_message(&self)
        -> impl Future<Output = Result<platform::Message, UserNextMessageError>>;
}

pub trait PlatformHookRequest {
    fn init(&self) -> Result<hooks::HookRequestInit, hooks::HookRequestError>;
    fn respond(&self, body: hooks::HookBody) -> Result<(), platform::SendError>;
}
