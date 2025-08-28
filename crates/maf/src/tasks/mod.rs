#[cfg(not(feature = "native"))]
mod wasi;

#[cfg(feature = "native")]
mod native {
    use std::future::IntoFuture;

    pub fn spawn<T: IntoFuture + 'static>(fut: T) -> tokio::task::JoinHandle<T::Output>
    where
        T::IntoFuture: Send,
        T::Output: Send,
    {
        tokio::spawn(fut.into_future())
    }
}

#[cfg(feature = "native")]
pub use native::*;
#[cfg(not(feature = "native"))]
pub use wasi::*;
