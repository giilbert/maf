use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use uuid::Uuid;

use crate::{
    app::AppState,
    bindings::bindgen::{self, maf::bindings::bindings},
    channel::BoundChannel,
    packet::RxPacket,
    tasks::Runtime,
    App, Channel,
};

pub struct UserListener {
    state: Arc<AppState>,
    future_user: bindgen::FutureUser,
}

impl UserListener {
    pub fn new(state: Arc<AppState>) -> Result<Self, bindgen::ListenError> {
        Ok(Self {
            state,
            future_user: bindgen::listen_user()?,
        })
    }

    pub fn next(&self) -> UserListenerNextFuture<'_> {
        UserListenerNextFuture {
            state: &self.state,
            listener: &self.future_user,
        }
    }
}

pub struct UserListenerNextFuture<'a> {
    state: &'a Arc<AppState>,
    listener: &'a bindgen::FutureUser,
}

impl UserListenerNextFuture<'_> {
    pub fn try_poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Result<Poll<User>, bindgen::ListenError> {
        match self.listener.get() {
            Ok(raw_user) => return Ok(Poll::Ready(User::new(self.state.clone(), raw_user))),
            Err(bindgen::ListenError::NotReady) => {
                let pollable = self.listener.subscribe()?;
                if pollable.ready() {
                    return self.try_poll(cx);
                } else {
                    Runtime::new_waker(cx, pollable, Some("listen user"));
                    Ok(Poll::Pending)
                }
            }
            Err(err) => return Err(err),
        }
    }
}

impl Future for UserListenerNextFuture<'_> {
    type Output = Result<User, bindgen::ListenError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.try_poll(cx) {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub meta: UserMeta,
    pub state: Arc<AppState>,
    inner: Arc<bindgen::User>,
}

#[derive(Debug, Clone)]
pub struct UserMeta {
    pub(crate) id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("channel is closed")]
    Closed,
    #[error("buffer is full")]
    BufferFull,
    #[error("failed to serialize message")]
    Serialize(#[from] serde_json::Error),
}

impl User {
    pub(crate) fn new(state: Arc<AppState>, raw: bindgen::User) -> Self {
        let raw_meta = raw.meta();
        Self {
            meta: UserMeta {
                id: Uuid::from_u64_pair(raw_meta.id.0, raw_meta.id.1),
            },
            state,
            inner: Arc::new(raw),
        }
    }

    pub(crate) fn listen_messages(&self) -> Result<UserMessageListener<'_>, bindgen::ListenError> {
        let future_message = self.inner.listen_message()?;

        Ok(UserMessageListener {
            user: self,
            listener: future_message,
        })
    }

    pub(crate) async fn handle_messages(&self, app: App) -> Result<(), bindgen::ListenError> {
        let messages = self.listen_messages()?;

        loop {
            let message = match messages.next().await {
                Ok(message) => message,
                Err(UserNextMessageError::Listen(bindgen::ListenError::Closed)) => {
                    break;
                }
                Err(e) => {
                    println!("warn: failed to listen for message: {e}");
                    continue;
                }
            };

            match app.handle_message(message).await {
                Ok(_) => {}
                Err(err) => {
                    println!("warn: failed to handle message: {err}");
                    continue;
                }
            }
        }

        Ok(())
    }

    // TODO: proper error handling
    pub(crate) fn send(&self, data: impl serde::Serialize) -> Result<(), SendError> {
        let text = serde_json::to_string(&data)?;
        self.inner.send(&bindgen::Message::Text(text))?;
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

pub struct UserNextMessageFuture<'a> {
    user: &'a User,
    listener: &'a bindings::FutureMessage,
}

#[derive(Debug, thiserror::Error)]
pub enum UserNextMessageError {
    #[error("failed to deserialize message")]
    Deserialize(#[from] serde_json::Error),
    #[error("failed to listen for message")]
    Listen(#[from] bindgen::ListenError),
}

impl<'a> UserNextMessageFuture<'a> {
    pub fn try_poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Result<Poll<UserMessage<'a>>, UserNextMessageError> {
        match self.listener.get() {
            Ok(raw_message) => match raw_message {
                bindgen::Message::Text(text) => {
                    return Ok(Poll::Ready(UserMessage {
                        user: self.user,
                        packet: serde_json::from_str(text.as_str())?,
                    }));
                }
                bindgen::Message::Binary(_bytes) => {
                    todo!("handle binary messages");
                }
            },
            Err(bindgen::ListenError::NotReady) => {
                let pollable = self.listener.subscribe()?;
                if pollable.ready() {
                    return self.try_poll(cx);
                } else {
                    Runtime::new_waker(cx, pollable, Some("listen user message"));
                    Ok(Poll::Pending)
                }
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }
}

impl<'a> Future for UserNextMessageFuture<'a> {
    type Output = Result<UserMessage<'a>, UserNextMessageError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.try_poll(cx) {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

#[derive(Debug)]
pub struct UserMessage<'a> {
    pub(crate) user: &'a User,
    pub(crate) packet: RxPacket,
}

pub(crate) struct UserMessageListener<'a> {
    user: &'a User,
    listener: bindings::FutureMessage,
}

impl<'a> UserMessageListener<'a> {
    pub fn next(&self) -> UserNextMessageFuture<'_> {
        UserNextMessageFuture {
            user: self.user,
            listener: &self.listener,
        }
    }
}

impl From<bindgen::SendError> for SendError {
    fn from(value: bindgen::SendError) -> Self {
        match value {
            bindgen::SendError::Closed => SendError::Closed,
            bindgen::SendError::BufferFull => SendError::BufferFull,
        }
    }
}
