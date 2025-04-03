use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{bindings::bindgen, tasks::Runtime};

pub(crate) struct UserListener {}

pub(crate) fn next_user() -> UserListener {
    UserListener::new()
}

impl UserListener {
    pub fn new() -> Self {
        let future_user = bindgen::listen_user()?;

        Self {}
    }
}

impl UserListener {
    pub fn try_poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Result<Poll<User>, bindgen::ListenError> {
        match self.future_user.get() {
            Ok(raw_user) => return Ok(Poll::Ready(User::new(raw_user))),
            Err(bindgen::ListenError::NotReady) => {
                let pollable = future_user.subscribe()?;
                if pollable.ready() {
                    return self.try_poll(cx);
                } else {
                    Runtime::current().add_pollable(pollable, cx.waker().clone());
                }
            }
            Err(err) => return Err(err),
        }

        Ok(Poll::Pending)
    }
}

impl Future for UserListener {
    type Output = Result<User, bindgen::ListenError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.try_poll(cx) {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

pub struct User {
    inner: bindgen::User,
}

impl User {
    pub fn new(inner: bindgen::User) -> Self {
        Self { inner }
    }
}
