use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use serde::de::DeserializeOwned;

pub struct RpcFunction {
    pub(crate) path: String,
    pub(crate) type_id: TypeId,
    pub(crate) handler: Box<dyn Fn(RpcRequest) -> anyhow::Result<RpcResponse> + Send + Sync>,
}

impl std::fmt::Debug for RpcFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcFunction")
            .field("path", &self.path)
            .field("type_id", &self.type_id)
            .finish()
    }
}

pub struct RpcRequest {
    path: String,
    data: Option<Vec<u8>>,
}

pub trait FromRequest
where
    Self: Sized,
{
    fn from_request(request: &RpcRequest) -> anyhow::Result<Self>;
}

#[derive(Debug)]
pub struct Body<T: DeserializeOwned>(pub T);

impl<T: DeserializeOwned> FromRequest for Body<T> {
    fn from_request(request: &RpcRequest) -> anyhow::Result<Self> {
        let data = serde_json::from_slice(
            &request
                .data
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("request body data already taken"))?,
        )?;
        Ok(Self(data))
    }
}

pub struct RpcResponse {
    data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct RpcStore {
    rpc_functions: HashMap<String, RpcFunction>,
}

impl RpcStore {
    pub fn add_rpc_function(&mut self, rpc_function: RpcFunction) {
        self.rpc_functions
            .insert(rpc_function.path.clone(), rpc_function);
    }
}

pub trait IntoRpcFunction<R, P> {
    fn into_rpc_function(self, path: String) -> RpcFunction;
}

impl<R, F: Send + Sync + Fn() -> R + 'static> IntoRpcFunction<R, ()> for F
where
    R: Send + serde::Serialize + 'static,
{
    fn into_rpc_function(self, path: String) -> RpcFunction {
        RpcFunction {
            path,
            type_id: self.type_id(),
            handler: Box::new(move |_request| {
                let response = self();
                Ok(RpcResponse {
                    data: serde_json::to_vec(&response)?,
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
        > IntoRpcFunction<R, ($($members),*)> for F
        where
            R: Send + Sync + serde::Serialize + 'static,
            $($members: Send + Sync + FromRequest + 'static),+
        {
            #[allow(non_snake_case)]
            fn into_rpc_function(self, path: String) -> RpcFunction {
                RpcFunction {
                    path,
                    type_id: self.type_id(),
                    handler: Box::new(move |request| {
                        let ($($members),+) = ($($members::from_request(&request)?),+);

                        let response = self($($members),+);

                        Ok(RpcResponse {
                            data: serde_json::to_vec(&response)?,
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
