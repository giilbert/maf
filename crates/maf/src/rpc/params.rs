use serde::de::DeserializeOwned;

use crate::callable::{CallableParam, SupportsAsync};

use super::{RpcError, RpcRequestContext, RpcRequestData, RpcRequestInit};

/// Extractor for RPC function parameters.
///
/// ```rust
/// use maf::prelude::*;
///
/// // A simple RPC that takes a single integer parameter.
/// async fn increment_counter(Params(counter): Params<i32>, test: Store<CounterStore>) -> i32 {
///     let mut store = test.write().await;
///     store.count += counter;
///     store.count
/// }
///
/// async fn set_counter(
///     // Multiple parameters can be extracted as a tuple.
///     Params((new_value, stop, reason)): Params<(i32, boolean, String)>,
///     test: Store<CounterStore>
/// ) {
///     let mut store = test.write().await;
///     println!("Setting counter to {} because {}", new_value, reason);
///     store.count = new_value;
///     if stop {
///         // Do something to stop the counter...
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Params<T: DeserializeOwned>(pub T);

#[derive(Debug, thiserror::Error)]
pub enum ParamsError {
    #[error("failed to deserialize params: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("request body data already consumed")]
    DataAlreadyConsumed,
}

impl<T: DeserializeOwned + Send + Sync> CallableParam<RpcRequestContext, RpcRequestInit>
    for Params<T>
{
    type Error = RpcError;

    async fn extract(
        ctx: &mut RpcRequestContext,
        init: &RpcRequestInit,
    ) -> Result<Self, Self::Error> {
        let data = match ctx.request.data.take().ok_or_else(|| {
            RpcError::ParamsError(init.method.clone(), ParamsError::DataAlreadyConsumed)
        })? {
            RpcRequestData::Typed(data) => serde_json::from_value(data)?,
        };

        Ok(Self(data))
    }
}

impl<T: DeserializeOwned> SupportsAsync for Params<T> {}
