pub mod models;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use anyhow::Context;
use models::{TypedRpcRequestPacket, TypedRpcResponsePacket};
use serde::de::DeserializeOwned;

pub struct RpcFunction {
    pub(crate) method: String,
    pub(crate) type_id: TypeId,
    pub(crate) handler:
        Box<dyn Fn(RpcRequest) -> anyhow::Result<TypedRpcResponsePacket> + Send + Sync>,
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
    id: u32,
    method: String,
    data: Option<RpcRequestData>,
}

#[derive(Debug)]
enum RpcRequestData {
    Typed(serde_json::Value),
}

pub trait FromRequest
where
    Self: Sized,
{
    fn from_request(request: &mut RpcRequest) -> anyhow::Result<Self>;
}

#[derive(Debug)]
pub struct Params<T: DeserializeOwned>(pub T);

impl<T: DeserializeOwned> FromRequest for Params<T> {
    fn from_request(request: &mut RpcRequest) -> anyhow::Result<Self> {
        let data = match request
            .data
            .take()
            .ok_or_else(|| anyhow::anyhow!("request body data already taken"))?
        {
            RpcRequestData::Typed(data) => {
                serde_json::from_value(data).context("failed to deserialize params")?
            }
        };

        Ok(Self(data))
    }
}

#[derive(Debug, Default)]
pub struct RpcStore {
    rpc_functions: HashMap<String, RpcFunction>,
}

impl RpcStore {
    pub fn add_rpc_function(&mut self, rpc_function: RpcFunction) {
        self.rpc_functions
            .insert(rpc_function.method.clone(), rpc_function);
    }

    pub fn handle_typed_rpc_request(
        &self,
        packet: TypedRpcRequestPacket,
    ) -> anyhow::Result<TypedRpcResponsePacket> {
        let method = packet.method;
        let rpc_function = self
            .rpc_functions
            .get(&method)
            .context("rpc function not found")?;

        let request = RpcRequest {
            id: packet.id,
            method,
            data: Some(RpcRequestData::Typed(packet.params)),
        };

        (rpc_function.handler)(request)
    }
}

pub trait IntoRpcFunction<Params, Returns> {
    fn into_rpc_function(self, method: String) -> RpcFunction;
}

impl<R, F: Send + Sync + Fn() -> R + 'static> IntoRpcFunction<(), R> for F
where
    R: Send + serde::Serialize + 'static,
{
    fn into_rpc_function(self, method: String) -> RpcFunction {
        RpcFunction {
            method,
            type_id: self.type_id(),
            handler: Box::new(move |request| {
                Ok(TypedRpcResponsePacket {
                    id: request.id,
                    result: serde_json::to_value(self())?,
                })
            }),
        }
    }
}

macro_rules! impl_rpc_fn {
    ($($members:ident),+) => {
        #[allow(unused_parens)]
        impl<
            R,
            $($members),*,
            F: Send + Sync + Fn($($members),+) -> R + 'static,
        > IntoRpcFunction<($($members),*), R> for F
        where
            R: Send + Sync + serde::Serialize + 'static,
            $($members: Send + Sync + FromRequest + 'static),+
        {
            #[allow(non_snake_case)]
            fn into_rpc_function(self, method: String) -> RpcFunction {
                RpcFunction {
                    method,
                    type_id: self.type_id(),
                    handler: Box::new(move |mut request| {
                        let ($($members),+) = ($($members::from_request(&mut request)?),+);

                        let result = serde_json::to_value(self($($members),+))?;

                        Ok(TypedRpcResponsePacket {
                            id: request.id,
                            result,
                        })
                    }),
                }
            }
        }
    }
}

impl_rpc_fn!(P1);
impl_rpc_fn!(P1, P2);
impl_rpc_fn!(P1, P2, P3);
impl_rpc_fn!(P1, P2, P3, P4);
impl_rpc_fn!(P1, P2, P3, P4, P5);
impl_rpc_fn!(P1, P2, P3, P4, P5, P6);
impl_rpc_fn!(P1, P2, P3, P4, P5, P6, P7);
impl_rpc_fn!(P1, P2, P3, P4, P5, P6, P7, P8);
