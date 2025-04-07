use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use uuid::Uuid;

use crate::{bindings::bindgen, tasks::Runtime};

pub struct UserListener {
    future_user: bindgen::FutureUser,
}

impl UserListener {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            future_user: bindgen::listen_user()?,
        })
    }

    pub fn next(&self) -> UserListenerNextFuture<'_> {
        UserListenerNextFuture {
            listener: &self.future_user,
        }
    }
}

pub struct UserListenerNextFuture<'a> {
    listener: &'a bindgen::FutureUser,
}

impl UserListenerNextFuture<'_> {
    pub fn try_poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Result<Poll<User>, bindgen::ListenError> {
        match self.listener.get() {
            Ok(raw_user) => return Ok(Poll::Ready(User::new(raw_user))),
            Err(bindgen::ListenError::NotReady) => {
                let pollable = self.listener.subscribe()?;
                if pollable.ready() {
                    return self.try_poll(cx);
                } else {
                    Runtime::new_waker(cx, pollable);
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
    inner: Arc<bindgen::User>,
}

#[derive(Debug, Clone)]
pub struct UserMeta {
    pub(crate) id: Uuid,
}

impl User {
    pub fn new(raw: bindgen::User) -> Self {
        let raw_meta = raw.meta();
        Self {
            meta: UserMeta {
                id: Uuid::from_u64_pair(raw_meta.id.0, raw_meta.id.1),
            },
            inner: Arc::new(raw),
        }
    }

    // TODO: proper error handling
    pub(crate) fn send(&self, data: impl serde::Serialize) -> anyhow::Result<()> {
        let text = serde_json::to_string(&data)?;
        self.inner
            .send(&bindgen::Message::Text(text))
            .map_err(|_| anyhow::anyhow!("Failed to send message"))
    }

    pub(crate) fn send_binary(&self, bytes: Vec<u8>) -> anyhow::Result<()> {
        self.inner
            .send(&bindgen::Message::Binary(bytes))
            .map_err(|_| anyhow::anyhow!("Failed to send binary message"))
    }
}

impl UserMeta {
    pub fn id(&self) -> Uuid {
        self.id
    }
}
