use std::sync::Arc;

use uuid::Uuid;

use crate::{
    app::AppState,
    channel::BoundChannel,
    packet::RxPacket,
    platform::{self, Message, PlatformUser, SendError},
    Channel,
};

#[derive(Debug, Clone)]
pub struct User {
    pub meta: UserMeta,
    pub state: Arc<AppState>,
    inner: Arc<platform::RawUser>,
}

#[derive(Debug, Clone)]
pub struct UserMeta {
    pub(crate) id: Uuid,
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
        }
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
