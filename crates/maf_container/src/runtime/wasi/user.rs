use anyhow::anyhow;
use axum::extract::ws::Message;
use tokio::sync::mpsc;
use wasmtime::component::Resource;
use wasmtime_wasi::async_trait;
use wasmtime_wasi_io::poll;

use crate::{api::connection::ConnectionHandle, container::ContainerData};

use super::{bindings, errors::ListenError};

pub struct User {
    pub handle: ConnectionHandle,
}

pub struct FutureUser {
    next_user: Option<ConnectionHandle>,
    channel: mpsc::Receiver<ConnectionHandle>,
}

impl FutureUser {
    pub fn new(container_data: &mut ContainerData) -> anyhow::Result<Self> {
        Ok(Self {
            next_user: None,
            channel: container_data
                .connection_rx
                .take()
                .ok_or_else(|| anyhow!(bindings::ListenError::AlreadyListening))?,
        })
    }
}

impl bindings::HostFutureUser for ContainerData {
    async fn drop(&mut self, user: Resource<FutureUser>) -> anyhow::Result<()> {
        let mut user = self.resources.delete(user)?;
        user.channel.close();
        Ok(())
    }

    async fn get(
        &mut self,
        future_user: Resource<FutureUser>,
    ) -> Result<Resource<User>, ListenError> {
        let future_user = self.resources.get_mut(&future_user)?;
        match future_user.next_user.take() {
            Some(handle) => Ok(self.resources.push(User { handle })?),
            None => Err(bindings::ListenError::NotReady.into()),
        }
    }

    async fn subscribe(
        &mut self,
        future_user: Resource<FutureUser>,
    ) -> Result<Resource<bindings::Pollable>, ListenError> {
        Ok(poll::subscribe(&mut self.resources, future_user)?)
    }
}

#[async_trait]
impl wasmtime_wasi::Pollable for FutureUser {
    async fn ready(&mut self) {
        let next_user = self.channel.recv().await;
        self.next_user = next_user;
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
        tracing::info!("drop(user {})", user.rep());
        Ok(())
    }

    async fn new(&mut self, id: (u64, u64)) -> anyhow::Result<Resource<bindings::User>> {
        todo!();
    }

    async fn send(
        &mut self,
        user: Resource<bindings::User>,
        message: bindings::Message,
    ) -> anyhow::Result<Result<(), ()>> {
        let user = self.resources.get_mut(&user)?;
        let message = match message {
            bindings::Message::Text(text) => Message::Text(text.into()),
            bindings::Message::Binary(bytes) => Message::Binary(bytes.into()),
        };
        user.handle.send(message).await?;
        Ok(Ok(()))
    }
}
