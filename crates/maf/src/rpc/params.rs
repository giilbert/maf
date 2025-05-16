use serde::de::DeserializeOwned;

use crate::callable::CallableParam;

use super::{RpcError, RpcRequestContext, RpcRequestData, RpcRequestInit};

#[derive(Debug)]
pub struct Params<T: DeserializeOwned>(pub T);

#[derive(Debug, thiserror::Error)]
pub enum ParamsError {
    #[error("failed to deserialize params: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("request body data already consumed")]
    DataAlreadyConsumed,
}

impl<T: DeserializeOwned> CallableParam<RpcRequestContext, RpcRequestInit> for Params<T> {
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
