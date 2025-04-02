use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::Runtime;

#[derive(Debug)]
pub struct SleepFuture {
    deadline: u64,
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("poll() called on SleepFuture");

        let pollable = wasi::clocks::monotonic_clock::subscribe_instant(self.deadline);
        if pollable.ready() {
            return Poll::Ready(());
        }

        Runtime::current().add_pollable(pollable, cx.waker().clone());

        Poll::Pending
    }
}

impl SleepFuture {
    pub fn new(deadline: u64) -> Self {
        Self { deadline }
    }
}
