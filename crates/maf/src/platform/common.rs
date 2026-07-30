//! Shared interfaces used by different platforms. Currently, this is only used for WASI and native
//! platforms, but it is designed to be extensible for other platforms in the future.
//!
//! #[cfg(..)] attributes are used to run code conditionally based on the target platform. e.g. no
//! trait objects are instantiated on runtime: there are no `Box<dyn Platform>`'s or
//! `impl Platform`'s anywhere. This is to ensure simplicity and avoid dynamic dispatch overhead.

use std::future::Future;

use maf_schemas::apps;

use crate::app::hooks;
use crate::platform::{self, AddKeyError, ListenError, SendError};
use crate::user::{UserMeta, UserNextMessageError};

/// Traits defining how a MAF app running should interact with the underlying platform (e.g. MAF
/// Platform, native server, etc.). This is almost a 1-1 translation of `maf.wit`.
#[allow(unused)]
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

    /// Used for type generation.
    fn report_app_schema(&self, schema: &str);

    fn set_meta(
        &self,
        visibility: apps::MetaVisibility,
        key: &str,
        value: &str,
    ) -> Option<apps::MetaEntry>;
    fn get_meta(&self, key: &str) -> Option<apps::MetaEntry>;
    fn delete_meta(&self, key: &str) -> Option<apps::MetaEntry>;
    fn list_meta(&self) -> Vec<(String, apps::MetaEntry)>;

    fn add_key(&self, key: String) -> Result<(), AddKeyError>;
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

    /// Disconnects the user from the MAF room.
    fn disconnect(&self) -> Result<(), SendError>;

    /// Returns true if the user has disconnected from the MAF room.
    fn is_disconnected(&self) -> bool;
}

pub trait PlatformHookRequest {
    fn init(&self) -> Result<hooks::HookRequestInit, hooks::HookRequestError>;
    fn respond(&self, body: hooks::HookBody) -> Result<(), platform::SendError>;
}
