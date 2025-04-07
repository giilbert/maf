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
        self.next_user = self.channel.recv().await;
    }
}

pub struct FutureMessage {
    next_message: Option<bindings::Message>,
    channel: mpsc::Receiver<bindings::Message>,
}

#[async_trait]
impl wasmtime_wasi::Pollable for FutureMessage {
    async fn ready(&mut self) {
        self.next_message = self.channel.recv().await;
    }
}

impl bindings::HostFutureMessage for ContainerData {
    async fn drop(&mut self, future_message: Resource<FutureMessage>) -> wasmtime::Result<()> {
        let mut future_message = self.resources.delete(future_message)?;
        future_message.channel.close();
        Ok(())
    }

    async fn get(
        &mut self,
        future_message: Resource<FutureMessage>,
    ) -> Result<bindings::Message, ListenError> {
        let future_message = self.resources.get_mut(&future_message)?;
        match future_message.next_message.take() {
            Some(handle) => Ok(handle),
            None => Err(bindings::ListenError::NotReady.into()),
        }
    }

    async fn subscribe(
        &mut self,
        future_message: Resource<FutureMessage>,
    ) -> Result<Resource<bindings::Pollable>, ListenError> {
        Ok(poll::subscribe(&mut self.resources, future_message)?)
    }
}

impl bindings::HostUser for ContainerData {
    async fn new(&mut self, id: (u64, u64)) -> anyhow::Result<Resource<bindings::User>> {
        todo!();
    }

    async fn drop(&mut self, user: Resource<User>) -> anyhow::Result<()> {
        tracing::info!("drop(user {})", user.rep());
        Ok(())
    }

    async fn meta(&mut self, user: Resource<User>) -> wasmtime::Result<bindings::UserMeta> {
        let user = self.resources.get_mut(&user)?;
        Ok(bindings::UserMeta {
            id: user.handle.id.as_u64_pair(),
        })
    }

    async fn listen_message(
        &mut self,
        user: Resource<User>,
    ) -> anyhow::Result<Resource<bindings::FutureMessage>, ListenError> {
        let message_rx = self.resources.get(&user)?.handle.take_message_rx().await?;
        Ok(self.resources.push_child(
            FutureMessage {
                channel: message_rx,
                next_message: None,
            },
            &user,
        )?)
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
        if let Err(_) = user.handle.send(message) {
            // tracing::error!("failed to send message: {e:?}");
            return Ok(Err(()));
        }

        Ok(Ok(()))
    }
}
