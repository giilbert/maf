use tokio::sync::mpsc;
use wasmtime::component::Resource;

use crate::container::ContainerData;

use super::{bindings, errors::ListenError, User};

pub struct FutureRequest {
    channel: mpsc::Receiver<()>,
}

impl bindings::HostFutureRequest for ContainerData {
    async fn drop(&mut self, request: Resource<bindings::FutureRequest>) -> anyhow::Result<()> {
        todo!();
    }

    async fn get(
        &mut self,
        request: Resource<bindings::FutureRequest>,
    ) -> Result<bindings::Request, ListenError> {
        todo!();
    }

    async fn subscribe(
        &mut self,
        request: Resource<bindings::FutureRequest>,
    ) -> Result<Resource<bindings::Pollable>, ListenError> {
        todo!();
    }
}
