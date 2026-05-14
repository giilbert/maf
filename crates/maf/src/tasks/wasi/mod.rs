mod gen_vec;
mod runtime;
mod task;
pub mod timers;
mod waker;

pub use futures_util;
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    task::Poll,
};
use wasi::io::poll::Pollable;

use runtime::JoinHandle;
pub use runtime::Runtime;
use timers::SleepFuture;

/// Spawns a new asynchronous task on the current runtime.
pub fn spawn<T: IntoFuture + 'static>(fut: T) -> JoinHandle<T::Output> {
    Runtime::current().spawn(fut)
}

/// Sleeps until the specified instant.
pub fn sleep_until(instant: std::time::Instant) -> SleepFuture {
    let duration = instant
        .checked_duration_since(std::time::Instant::now())
        .unwrap_or_default();
    SleepFuture::new(duration)
}

/// Sleeps for the specified duration.
pub fn sleep(duration: std::time::Duration) -> SleepFuture {
    SleepFuture::new(duration)
}

#[must_use]
pub struct WaitForPollable {
    pollable: Option<Pollable>,
    name: Option<&'static str>,
}

impl WaitForPollable {
    /// Optionally sets a name for the pollable being waited on, which can be used for debugging
    /// purposes.
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }
}

/// Waits for a [`Pollable`] to become ready. Useful for waiting on WASI I/O operations where
/// getting a result from I/O is handled by a separate method.
pub fn wait_for(pollable: Pollable) -> WaitForPollable {
    WaitForPollable {
        pollable: Some(pollable),
        name: None,
    }
}

impl Future for WaitForPollable {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if self.pollable.as_ref().map(|p| p.ready()).unwrap_or(true) {
            Poll::Ready(())
        } else {
            Runtime::new_waker(
                cx,
                self.pollable.take().expect("pollable not set"),
                self.name.clone(),
            );

            Poll::Pending
        }
    }
}
