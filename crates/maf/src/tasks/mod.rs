mod gen_vec;
mod runtime;
mod task;
pub mod timers;
mod waker;

pub use futures_util;
use std::future::IntoFuture;

use runtime::JoinHandle;
pub use runtime::Runtime;
use timers::SleepFuture;

pub fn spawn<T: IntoFuture + 'static>(fut: T) -> JoinHandle<T::Output> {
    Runtime::current().spawn(fut)
}

pub fn sleep_until(instant: std::time::Instant) -> SleepFuture {
    let duration = instant
        .checked_duration_since(std::time::Instant::now())
        .unwrap_or_default();
    SleepFuture::new(duration)
}

pub fn sleep(duration: std::time::Duration) -> SleepFuture {
    SleepFuture::new(duration)
}

// pub fn sleep_until(deadline: u64) -> SleepFuture {
//     SleepFuture::new(deadline)
// }
