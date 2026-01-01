//! Abstractions for connected users.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use maf_schemas::packet::RxPacket;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
    app::AppState,
    callable::{CallableFetch, CallableParam, SupportsAsync},
    channel::BoundChannel,
    platform::{self, Message, PlatformUser, SendError},
    App, Channel,
};

/// Represents a connected user.
///
/// ## Extractor
/// The [`User`] type can be used as an extractor in RPC functions to access information about the
/// user that made the request. For example:
/// ```rust
/// use maf::prelude::*;
///
/// async fn greet_user(user: User) -> String {
///     format!("Hello, user with ID: {}", user.meta().id())
/// }
///
/// fn build() -> App {
///     App::builder()
///         .rpc("greet_user", greet_user)
///         .build()
/// }
///
/// maf::register!(build);
/// ```
#[derive(Debug, Clone)]
pub struct User {
    meta: UserMeta,
    state: Arc<AppState>,
    inner: Arc<platform::RawUser>,
    auth_deserialized_cached: Arc<RwLock<Option<Arc<dyn std::any::Any + Send + Sync>>>>,
}

/// MAF user metadata
#[derive(Debug, Clone)]
pub struct UserMeta {
    pub id: Uuid,
    pub auth: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum UserNextMessageError {
    #[error("failed to deserialize message")]
    Deserialize(#[from] serde_json::Error),
    #[error("failed to listen for message")]
    Listen(#[from] platform::ListenError),
}

impl User {
    pub(crate) fn new(state: Arc<AppState>, raw: platform::RawUser) -> Self {
        Self {
            meta: raw.meta(),
            state,
            inner: Arc::new(raw),
            auth_deserialized_cached: Arc::new(RwLock::new(None)),
        }
    }

    /// Returns a reference to the user's metadata.
    pub fn meta(&self) -> &UserMeta {
        &self.meta
    }

    /// Tries to decode the user's authentication information, panicking if it does not exist or
    /// match the expected type.
    pub fn auth<T: DeserializeOwned + Send + Sync + 'static>(&self) -> Arc<T> {
        // If we have a cached version, return it
        if let Some(Ok(cached)) = self
            .auth_deserialized_cached
            .read()
            .expect("poisoned lock")
            .as_ref()
            .map(|arc_any| arc_any.clone().downcast::<T>())
        {
            return cached;
        }

        let auth = self.meta.auth.as_ref().expect("user is not authenticated");
        let auth_arc: Arc<T> = Arc::new(
            serde_json::from_value::<T>(auth.clone())
                .expect("failed to deserialize user auth data"),
        );

        self.auth_deserialized_cached
            .write()
            .expect("poisoned lock")
            .replace(auth_arc.clone());

        auth_arc
    }

    /// Awaits the next message the user has sent from a message buffer.
    ///
    /// If `Err(UserNextMessageError::Listen(ListenError::Closed))` is returned, the user has
    /// disconnected and no further messages can be expected.
    pub(crate) async fn next_message(&self) -> Result<UserMessage<'_>, UserNextMessageError> {
        let message = self.inner.next_message().await?;

        Ok(UserMessage {
            user: self,
            packet: match message {
                platform::Message::Text(text) => {
                    serde_json::from_str(&text).map_err(UserNextMessageError::Deserialize)?
                }
                platform::Message::Binary(data) => todo!("handle binary messages: {data:?}"),
            },
        })
    }

    // TODO: proper error handling
    pub(crate) fn send(&self, data: impl serde::Serialize) -> Result<(), SendError> {
        let text = serde_json::to_string(&data)?;
        self.inner.send(Message::Text(text))?;
        Ok(())
    }

    pub fn channel<T>(&self, name: impl ToString) -> BoundChannel<T> {
        let name = name.to_string();
        BoundChannel::new(Channel::new(self.state.clone(), name), &self)
    }
}

impl UserMeta {
    pub fn id(&self) -> Uuid {
        self.id
    }
}

#[derive(Debug)]
pub struct UserMessage<'a> {
    pub(crate) user: &'a User,
    pub(crate) packet: RxPacket,
}

/// An extractor to access connected users.
pub struct Users {
    app: App,
}

impl Users {
    pub(crate) fn new(app: App) -> Self {
        Self { app }
    }

    /// Returns a list of all currently connected users.
    ///
    /// ## Example
    /// ```rust
    /// use maf::prelude::*;
    ///
    /// // This RPC function returns a list of all connected user IDs.
    /// async fn list_users(users: Users) -> Vec<Uuid> {
    ///     let all_users = users.all().await;
    ///     all_users.keys().cloned().collect()
    /// }
    ///
    /// fn build() -> App {
    ///     App::builder()
    ///         .rpc("list_users", list_users)
    ///         .build()
    /// }
    ///
    /// maf::register!(build);
    /// ```
    pub async fn all(&self) -> HashMap<Uuid, User> {
        // TODO: not clone
        let users = self.app.inner.state.users.read().await;
        users.clone()
    }

    /// Returns the user with the given ID, if they are connected. Otherwise, returns [`None`].
    ///
    /// ## Example
    /// ```rust
    /// use maf::prelude::*;
    ///
    /// // This RPC function checks if a user with the given ID is connected.
    /// async fn is_user_connected(users: Users, Params(user_id): Params<Uuid>) -> bool {
    ///     users.get(&user_id).await.is_some()
    /// }
    ///
    /// fn build() -> App {
    ///     App::builder()
    ///         .rpc("is_user_connected", is_user_connected)
    ///         .build()
    /// }
    ///
    /// maf::register!(build);
    /// ```
    pub async fn get(&self, id: &Uuid) -> Option<User> {
        let users = self.app.inner.state.users.read().await;
        users.get(id).cloned()
    }

    /// Returns the number of currently connected users.
    ///
    /// ## Example
    /// ```rust
    /// use maf::prelude::*;
    ///
    /// fn build() -> App {
    ///     App::builder()
    ///         // Sends the number of connected users as a select that automatically updates.
    ///         .select("connected_user_count", |users: Users| async move {
    ///             users.count().await
    ///         })
    ///        .build()
    /// }
    ///
    /// maf::register!(build);
    /// ```
    pub async fn count(&self) -> usize {
        let users = self.app.inner.state.users.read().await;
        users.len()
    }
}

impl<Ctx, Init> CallableParam<Ctx, Init> for Users
where
    Ctx: CallableFetch<App>,
    Init: Send + Sync,
{
    type Error = std::convert::Infallible;
    async fn extract(ctx: &mut Ctx, _init: &Init) -> Result<Self, Self::Error> {
        let app = ctx.fetch();
        Ok(Users::new(app))
    }
}

impl SupportsAsync for Users {}
