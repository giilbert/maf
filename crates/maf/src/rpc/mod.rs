pub mod models;
mod params;

use std::{any::TypeId, collections::HashMap};

pub use params::Params;

use models::{TypedRpcRequestPacket, TypedRpcResponsePacket};
use params::ParamsError;

use crate::{
    callable::{AnyCallable, CallableParam},
    App, SendError, User,
};

pub struct RpcFunction {
    pub(crate) method: String,
    pub(crate) type_id: TypeId,
    pub(crate) handler: AnyCallable<RpcRequestContext, TypedRpcResponsePacket, RpcError>,
}

impl std::fmt::Debug for RpcFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcFunction")
            .field("method", &self.method)
            .field("type_id", &self.type_id)
            .finish()
    }
}

#[derive(Debug)]
pub struct RpcRequest {
    pub id: u32,
    pub user: User,
    pub data: Option<RpcRequestData>,
}

#[derive(Debug)]
pub enum RpcRequestData {
    Typed(serde_json::Value),
}

pub struct RpcRequestContext {
    pub app: App,
    pub request: RpcRequest,
}

#[derive(Debug, Default)]
pub struct RpcStore {
    rpc_functions: HashMap<String, RpcFunction>,
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("rpc method `{0}` not found")]
    MethodNotFound(String),
    #[error("rpc function in `{0}` error: {1}")]
    FunctionError(String, anyhow::Error),

    #[error("rpc response serialization error: {0}")]
    ResponseSerializationError(#[from] serde_json::Error),
    #[error("rpc response error: {0}")]
    ResponseError(#[from] SendError),

    #[error("rpc params in `{0}` error: {1}")]
    ParamsError(String, ParamsError),

    #[error("other error: {0}")]
    Other(#[from] anyhow::Error),

    #[error("infalliable error: {0}")]
    Infalliable(#[from] std::convert::Infallible),
}

impl RpcStore {
    pub fn add_rpc_function(&mut self, rpc_function: RpcFunction) {
        self.rpc_functions
            .insert(rpc_function.method.clone(), rpc_function);
    }

    pub async fn handle_typed_rpc_request(
        &self,
        app: App,
        user: &User,
        packet: TypedRpcRequestPacket,
    ) -> Result<TypedRpcResponsePacket, RpcError> {
        let method = packet.method;
        let rpc_function = self
            .rpc_functions
            .get(&method)
            .ok_or_else(|| RpcError::MethodNotFound(method))?;

        let request = RpcRequest {
            id: packet.id,
            user: user.clone(),
            data: Some(RpcRequestData::Typed(packet.params)),
        };

        let res = (rpc_function.handler)(RpcRequestContext { app, request });
        tokio::pin!(res);

        res.await
    }
}

pub const fn check_rpc_parameter<T: CallableParam<RpcRequestContext, String>>() {}

#[cfg(test)]
mod tests {
    use crate::{Store, StoreData};

    use super::*;

    fn type_checks() {
        check_rpc_parameter::<Params<i32>>();
        check_rpc_parameter::<Params<(i32, String)>>();

        struct T {}

        impl StoreData for T {
            type Data = i32;

            fn init() -> Self::Data {
                42
            }
        }

        check_rpc_parameter::<Store<T>>();
    }
}
