use anyhow::anyhow;
use tokio::sync::mpsc;
use wasmtime::component::Resource;
use wasmtime_wasi::async_trait;
use wasmtime_wasi_io::poll;

use crate::container::{connection::ConnectionHandle, ContainerData};

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
