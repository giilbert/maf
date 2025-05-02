mod from_request;
pub mod models;
mod params;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
};

pub use from_request::FromRequest;
pub use params::Params;

use anyhow::Context;
use models::{TypedRpcRequestPacket, TypedRpcResponsePacket};

use crate::app::AppState;

pub struct RpcFunction {
    pub(crate) method: String,
    pub(crate) type_id: TypeId,
    pub(crate) handler: Box<
        dyn Fn(
                Arc<AppState>,
                RpcRequest,
            ) -> anyhow::Result<
                Pin<Box<dyn Future<Output = anyhow::Result<TypedRpcResponsePacket>>>>,
            > + Send
            + Sync,
    >,
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

#[derive(Debug, Default)]
pub struct RpcStore {
    rpc_functions: HashMap<String, RpcFunction>,
}

impl RpcStore {
    pub fn add_rpc_function(&mut self, rpc_function: RpcFunction) {
        self.rpc_functions
            .insert(rpc_function.method.clone(), rpc_function);
    }

    pub async fn handle_typed_rpc_request(
        &self,
        state: Arc<AppState>,
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

        let res = (rpc_function.handler)(state, request)?;
        tokio::pin!(res);
        res.await
    }
}

pub trait IntoRpcFunction<Params, Returns> {
    fn into_rpc_function(self, method: String) -> RpcFunction;
}

impl<R, F> IntoRpcFunction<(), R> for F
where
    R: Send + serde::Serialize + 'static,
    F: Fn() -> R + Send + Sync + Copy + 'static,
{
    fn into_rpc_function(self, method: String) -> RpcFunction {
        RpcFunction {
            method,
            type_id: self.type_id(),
            handler: Box::new(move |_state, request| {
                Ok(Box::pin(async move {
                    Ok(TypedRpcResponsePacket {
                        id: request.id,
                        result: serde_json::to_value(self())?,
                    })
                }))
            }),
        }
    }
}

impl<R, Fut, F> IntoRpcFunction<PhantomData<()>, R> for F
where
    R: Send + serde::Serialize + 'static,
    F: Fn() -> Fut + Send + Sync + Copy + 'static,
    Fut: Future<Output = R> + Send + 'static,
{
    fn into_rpc_function(self, method: String) -> RpcFunction {
        RpcFunction {
            method,
            type_id: self.type_id(),
            handler: Box::new(move |_state, request| {
                Ok(Box::pin(async move {
                    let result = self().await;
                    Ok(TypedRpcResponsePacket {
                        id: request.id,
                        result: serde_json::to_value(result)?,
                    })
                }))
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
            F: Send + Sync + Fn($($members),+) -> R + Send + Sync + Copy + 'static,
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
                    handler: Box::new(move |state, mut request| {
                        Ok(Box::pin(async move {
                            let ($($members),+) = (
                                $($members::from_request(&state, &mut request).await?),+
                            );

                            let result = serde_json::to_value(self($($members),+))?;

                            Ok(TypedRpcResponsePacket {
                                id: request.id,
                                result,
                            })
                        }))
                    }),
                }
            }
        }

        impl<
            R,
            Fut,
            $($members),*,
            F: Fn($($members),+) -> Fut + Send + Sync + Copy + 'static
        > IntoRpcFunction<PhantomData<($($members),*,)>, R> for F
        where
            R: Send + Sync + serde::Serialize + 'static,
            $($members: Send + Sync + FromRequest + 'static),+,
            Fut: Future<Output = R> + Send + 'static,
        {
            #[allow(non_snake_case)]
            fn into_rpc_function(self, method: String) -> RpcFunction {
                RpcFunction {
                    method,
                    type_id: self.type_id(),
                    handler: Box::new(move |state, mut request| {
                        Ok(Box::pin(async move {
                            #[allow(unused_parens)]
                            let ($($members),+) = (
                                $($members::from_request(&state, &mut request).await?),+
                            );
                            let result = serde_json::to_value(self($($members),+).await)?;

                            Ok(TypedRpcResponsePacket {
                                id: request.id,
                                result,
                            })
                        }))
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
