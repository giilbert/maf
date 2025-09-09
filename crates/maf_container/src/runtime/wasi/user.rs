use tokio::sync::mpsc;
use wasmtime::component::Resource;
use wasmtime_wasi::async_trait;
use wasmtime_wasi_io::poll;

use crate::{container::ContainerData, interface::BoxedConnection};

use super::{bindings, errors::ListenError};

pub struct UserImpl {
    pub connection: BoxedConnection,
}

pub struct FutureUserImpl {
    next_user: Option<BoxedConnection>,
    channel: mpsc::Receiver<BoxedConnection>,
}

impl FutureUserImpl {
    pub fn new(container_data: &mut ContainerData) -> Result<Self, bindings::ListenError> {
        Ok(Self {
            next_user: None,
            channel: container_data
                .connection_rx
                .take()
                .ok_or_else(|| bindings::ListenError::AlreadyListening)?,
        })
    }
}

impl bindings::HostFutureUser for ContainerData {
    async fn drop(&mut self, user: Resource<FutureUserImpl>) -> anyhow::Result<()> {
        let mut future_user = self.resources.delete(user)?;
        future_user.channel.close();
        Ok(())
    }

    async fn get(
        &mut self,
        future_user: Resource<FutureUserImpl>,
    ) -> Result<Resource<UserImpl>, ListenError> {
        let future_user = self.resources.get_mut(&future_user)?;
        match future_user.next_user.take() {
            Some(handle) => {
                self.update_last_activity();
                Ok(self.resources.push(UserImpl { connection: handle })?)
            }
            None => Err(bindings::ListenError::NotReady.into()),
        }
    }

    async fn subscribe(
        &mut self,
        future_user: Resource<FutureUserImpl>,
    ) -> Result<Resource<bindings::Pollable>, ListenError> {
        Ok(poll::subscribe(&mut self.resources, future_user)?)
    }
}

#[async_trait]
impl wasmtime_wasi::p2::Pollable for FutureUserImpl {
    async fn ready(&mut self) {
        self.next_user = self.channel.recv().await;
    }
}

pub struct FutureMessageImpl {
    next_message: Option<bindings::Message>,
    channel: mpsc::Receiver<bindings::Message>,
}

#[async_trait]
impl wasmtime_wasi::p2::Pollable for FutureMessageImpl {
    async fn ready(&mut self) {
        self.next_message = self.channel.recv().await;
    }
}

impl bindings::HostFutureMessage for ContainerData {
    async fn drop(&mut self, future_message: Resource<FutureMessageImpl>) -> wasmtime::Result<()> {
        let mut future_message = self.resources.delete(future_message)?;
        future_message.channel.close();
        Ok(())
    }

    async fn get(
        &mut self,
        future_message: Resource<FutureMessageImpl>,
    ) -> Result<bindings::Message, ListenError> {
        let future_message = self.resources.get_mut(&future_message)?;

        if future_message.channel.is_closed() {
            return Err(bindings::ListenError::Closed.into());
        }

        match future_message.next_message.take() {
            Some(handle) => {
                self.update_last_activity();
                Ok(handle)
            }
            None => Err(bindings::ListenError::NotReady.into()),
        }
    }

    async fn subscribe(
        &mut self,
        future_message: Resource<FutureMessageImpl>,
    ) -> Result<Resource<bindings::Pollable>, ListenError> {
        Ok(poll::subscribe(&mut self.resources, future_message)?)
    }
}

impl bindings::HostUser for ContainerData {
    async fn drop(&mut self, user: Resource<UserImpl>) -> anyhow::Result<()> {
        tracing::info!("drop(user {})", user.rep());
        Ok(())
    }

    async fn meta(&mut self, user: Resource<UserImpl>) -> wasmtime::Result<bindings::UserMeta> {
        let user = self.resources.get_mut(&user)?;
        Ok(bindings::UserMeta {
            id: user.connection.id().as_u64_pair(),
        })
    }

    async fn listen_message(
        &mut self,
        user: Resource<bindings::User>,
    ) -> anyhow::Result<Resource<bindings::FutureMessage>, ListenError> {
        let message_rx = self
            .resources
            .get(&user)?
            .connection
            .get_message_channel()
            .await?;

        Ok(self.resources.push(FutureMessageImpl {
            channel: message_rx,
            next_message: None,
        })?)
    }

    async fn send(
        &mut self,
        user: Resource<bindings::User>,
        message: bindings::Message,
    ) -> anyhow::Result<Result<(), bindings::SendError>> {
        let user = self.resources.get_mut(&user)?;

        Ok(user.connection.send(message))
    }
}
