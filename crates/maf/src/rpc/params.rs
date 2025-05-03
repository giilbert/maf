use anyhow::Context;
use serde::de::DeserializeOwned;

use crate::App;

use super::{FromRequest, RpcRequest, RpcRequestData};

#[derive(Debug)]
pub struct Params<T: DeserializeOwned>(pub T);

impl<T: DeserializeOwned> FromRequest for Params<T> {
    async fn from_request(_app: &App, request: &mut RpcRequest) -> anyhow::Result<Self> {
        let data = match request
            .data
            .take()
            .ok_or_else(|| anyhow::anyhow!("request body data already consumed"))?
        {
            RpcRequestData::Typed(data) => {
                serde_json::from_value(data).context("failed to deserialize params")?
            }
        };

        Ok(Self(data))
    }
}
