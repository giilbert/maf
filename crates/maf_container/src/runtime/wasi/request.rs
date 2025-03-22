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

impl bindings::HostUser for ContainerData {
    async fn get_meta(
        &mut self,
        user: Resource<User>,
    ) -> wasmtime::Result<Result<bindings::UserMeta, ()>> {
        todo!()
    }

    async fn drop(&mut self, user: Resource<User>) -> anyhow::Result<()> {
        todo!();
    }

    async fn new(&mut self, id: (u64, u64)) -> anyhow::Result<Resource<bindings::User>> {
        todo!();
    }

    async fn send(
        &mut self,
        user: Resource<bindings::User>,
        bytes: Vec<u8>,
    ) -> anyhow::Result<Result<(), ()>> {
        todo!();
    }
}
