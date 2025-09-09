use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use wasmtime::component::Resource;
use wasmtime_wasi_io::poll;

use crate::ContainerData;

use super::{
    bindings::{self, HookRequestInit},
    errors::ListenError,
};

pub struct HookRequest {
    init: Option<bindings::HookRequestInit>,
    response_tx: Option<oneshot::Sender<bindings::HookBody>>,
}

pub struct FutureHookRequest {
    next_request: Option<HookRequest>,
    channel: mpsc::Receiver<HookRequest>,
}

impl FutureHookRequest {
    pub fn new(container_data: &mut ContainerData) -> anyhow::Result<Self> {
        Ok(Self {
            next_request: None,
            channel: container_data
                .hook_request_rx
                .take()
                .ok_or_else(|| anyhow!(bindings::ListenError::AlreadyListening))?,
        })
    }
}

impl bindings::HostFutureHookRequest for ContainerData {
    async fn drop(
        &mut self,
        future_hook_request: Resource<bindings::FutureHookRequest>,
    ) -> anyhow::Result<()> {
        let mut future_hook_request = self.resources.delete(future_hook_request)?;
        future_hook_request.channel.close();
        Ok(())
    }

    async fn get(
        &mut self,
        future_hook_request: Resource<bindings::FutureHookRequest>,
    ) -> Result<Resource<bindings::HookRequest>, ListenError> {
        let future_hook_request = self.resources.get_mut(&future_hook_request)?;
        match future_hook_request.next_request.take() {
            Some(hook_request) => {
                self.update_last_activity();
                Ok(self.resources.push(hook_request)?)
            }
            None => Err(bindings::ListenError::NotReady.into()),
        }
    }

    async fn subscribe(
        &mut self,
        future_hook_request: Resource<bindings::FutureHookRequest>,
    ) -> Result<Resource<bindings::Pollable>, ListenError> {
        Ok(poll::subscribe(&mut self.resources, future_hook_request)?)
    }
}

#[async_trait]
impl wasmtime_wasi::p2::Pollable for FutureHookRequest {
    async fn ready(&mut self) {
        self.next_request = self.channel.recv().await;
    }
}

impl bindings::HostHookRequest for ContainerData {
    async fn init(
        &mut self,
        hook_request: Resource<HookRequest>,
    ) -> anyhow::Result<Result<bindings::HookRequestInit, bindings::HookRequestError>> {
        Ok(Ok(self
            .resources
            .get_mut(&hook_request)?
            .init
            .take()
            .ok_or_else(|| bindings::HookRequestError::InitConsumed)?))
    }

    async fn respond(
        &mut self,
        hook_request: Resource<HookRequest>,
        response: bindings::HookBody,
    ) -> anyhow::Result<Result<(), bindings::SendError>> {
        let hook_request = self.resources.get_mut(&hook_request)?;
        Ok(hook_request.respond(response).await)
    }

    async fn drop(
        &mut self,
        hook_request: Resource<bindings::HookRequest>,
    ) -> wasmtime::Result<()> {
        let _hook_request = self.resources.delete(hook_request)?;
        Ok(())
    }
}

impl HookRequest {
    pub fn new(init: HookRequestInit, response_tx: oneshot::Sender<bindings::HookBody>) -> Self {
        Self {
            init: Some(init),
            response_tx: Some(response_tx),
        }
    }

    pub async fn respond(
        &mut self,
        response: bindings::HookBody,
    ) -> Result<(), bindings::SendError> {
        self.response_tx
            .take()
            .expect("request already responded to")
            .send(response)
            .map_err(|_| bindings::SendError::Closed)
    }
}
