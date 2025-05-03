use serde::de::DeserializeOwned;

use crate::App;

use super::{FromRequest, RpcRequest, RpcRequestData};

#[derive(Debug)]
pub struct Params<T: DeserializeOwned>(pub T);

#[derive(Debug, thiserror::Error)]
pub enum ParamsError {
    #[error("failed to deserialize params: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("request body data already consumed")]
    DataAlreadyConsumed,
}

impl<T: DeserializeOwned> FromRequest for Params<T> {
    type Error = ParamsError;

    async fn from_request(_app: &App, request: &mut RpcRequest) -> Result<Self, Self::Error> {
        let data = match request
            .data
            .take()
            .ok_or(ParamsError::DataAlreadyConsumed)?
        {
            RpcRequestData::Typed(data) => serde_json::from_value(data)?,
        };

        Ok(Self(data))
    }
}
