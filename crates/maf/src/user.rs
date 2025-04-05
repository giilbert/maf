use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

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

#[derive(Clone)]
pub struct User {
    inner: Arc<bindgen::User>,
}

impl User {
    pub fn new(raw: bindgen::User) -> Self {
        Self {
            inner: Arc::new(raw),
        }
    }

    // TODO: proper error handling
    pub fn send(&self, data: impl serde::Serialize) -> Result<(), ()> {
        let text = serde_json::to_string(&data).map_err(|_| ())?;
        self.inner.send(&bindgen::Message::Text(text))
    }

    pub fn send_binary(&self, bytes: Vec<u8>) -> Result<(), ()> {
        self.inner.send(&bindgen::Message::Binary(bytes))
    }
}
