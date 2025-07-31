//! This module provides the RPC (Remote Procedure Call) functionality for the application.
//!
//! It allows a MAF client to call functions on the server and receive responses.
//!
//! ## Parameters
//! An RPC function can accept parameters in the form of [`Params<T>`], where `T` is the type of the
//! parameter. If multiple inputs are needed, they can be passed as a tuple, e.g.
//! `Params<(i32, String)>`. The parameters will be deserialized from the JSON request body, leaving
//! it up to the client to ensure the correct types are sent and are in the correct order.
//!
//! ## Return Values
//! The return value of an RPC function can be any type that implements [`serde::Serialize`]. The
//! response will be serialized to JSON and sent back to the client.
//!
//! ## Additional APIs
//! An RPC function declaration can take in additional parameters which can be used to access
//! additional MAF APIs:
//!
//! - [`crate::App`]: The app instance that the RPC function is running in.
//! - [`crate::User`]: The user that made the request.
//! - [`crate::Store<T>`]: A store instance that can be used to access shared data.
//! - [`crate::Channel<T>`]: A channel instance that can be used to send messages to clients.

pub mod models;
mod params;

use std::{any::TypeId, collections::HashMap};

pub use params::Params;

use models::{TypedRpcRequestPacket, TypedRpcResponsePacket};
use params::ParamsError;

use crate::{
    callable::{AnyCallable, CallableFetch},
    typed::RpcDesc,
    App, SendError, StateError, User,
};

pub struct RpcFunction {
    pub(crate) method: String,
    pub(crate) type_id: TypeId,
    pub(crate) handler: AnyCallable<RpcRequestContext, TypedRpcResponsePacket, RpcError>,
    pub(crate) desc: RpcDesc,
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

pub struct RpcRequestInit {
    pub method: String,
}

#[derive(Debug, Default)]
pub struct RpcStore {
    pub(crate) inner: HashMap<String, RpcFunction>,
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

    #[error("state error: {0}")]
    State(#[from] StateError),

    #[error("infalliable error: {0}")]
    Infalliable(#[from] std::convert::Infallible),
}

impl RpcStore {
    pub fn add_rpc_function(&mut self, rpc_function: RpcFunction) {
        self.inner.insert(rpc_function.method.clone(), rpc_function);
    }

    pub async fn handle_typed_rpc_request(
        &self,
        app: App,
        user: &User,
        packet: TypedRpcRequestPacket,
    ) -> Result<TypedRpcResponsePacket, RpcError> {
        let method = packet.method;
        let rpc_function = self
            .inner
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

impl CallableFetch<App> for RpcRequestContext {
    fn fetch(&self) -> App {
        self.app.clone()
    }
}

impl CallableFetch<User> for RpcRequestContext {
    fn fetch(&self) -> User {
        self.request.user.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::{callable::CallableParam, Store, StoreData};

    use super::*;

    #[test]
    fn type_checks() {
        const fn check_rpc_parameter<T: CallableParam<RpcRequestContext, RpcRequestInit>>() {}

        check_rpc_parameter::<Params<i32>>();
        check_rpc_parameter::<Params<(i32, String)>>();

        struct T {}

        impl StoreData for T {
            type Select = ();

            fn init() -> Self {
                T {}
            }

            fn select(&self, _user: &User) -> &Self::Select {
                &()
            }
        }

        check_rpc_parameter::<Store<T>>();
    }
}
