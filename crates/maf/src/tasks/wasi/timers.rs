use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use wasi::clocks::monotonic_clock::{self, Pollable};

use super::Runtime;

#[derive(Debug)]
pub struct SleepFuture {
    pollable: Option<Pollable>,
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.pollable.as_ref().map(|p| p.ready()).unwrap_or(true) {
            Poll::Ready(())
        } else {
            Runtime::new_waker(
                cx,
                self.pollable.take().expect("pollable not set"),
                Some("sleep"),
            );
            Poll::Pending
        }
    }
}

impl SleepFuture {
    pub fn new(duration: Duration) -> Self {
        let pollable = monotonic_clock::subscribe_duration(duration.as_nanos() as u64);
        Self {
            pollable: Some(pollable),
        }
    }
}
