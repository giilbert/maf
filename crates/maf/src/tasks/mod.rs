mod runtime;
pub mod timers;
mod waker;

use std::future::IntoFuture;

use runtime::JoinHandle;
pub use runtime::Runtime;
use timers::SleepFuture;

pub fn spawn<T: IntoFuture + 'static>(fut: T) -> JoinHandle<T::Output> {
    let runtime = Runtime::current();
    runtime.spawn(fut)
}

pub fn sleep_until(deadline: u64) -> SleepFuture {
    SleepFuture::new(deadline)
}
