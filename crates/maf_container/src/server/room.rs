use rand::Rng as _;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    BoxedConnection, Connection, Container, ContainerRuntime,
    wasi::{
        HookRequest,
        bindings::{self, HookRequestCaller, HookRequestInit},
    },
};

use super::Bundle;

pub type RoomId = Uuid;

#[derive(Debug, Clone)]
pub struct Room {
    pub id: Uuid,
    connection_tx: mpsc::Sender<BoxedConnection>,
    hooks_request_tx: mpsc::Sender<HookRequest>,
}

impl Room {
    pub async fn new(
        container_runtime: &ContainerRuntime,
        bundle: Bundle,
    ) -> anyhow::Result<(Self, Container)> {
        let id = Uuid::new_v4();
        let container =
            Container::load_from_binary(&container_runtime, bundle.wasm_module, id).await?;

        Ok((
            Self {
                id,
                connection_tx: container.store.data().connection_tx.clone(),
                hooks_request_tx: container.store.data().hook_request_tx.clone(),
            },
            container,
        ))
    }

    pub async fn add_connection(&self, connection: impl Connection) -> anyhow::Result<()> {
        match self.connection_tx.send(Box::new(connection)).await {
            Ok(_) => tracing::info!("connection added to room {}", self.id),
            Err(_) => anyhow::bail!("failed to add connection to room {}", self.id),
        }

        Ok(())
    }

    pub async fn call_hook(
        &self,
        caller: HookRequestCaller,
        method: String,
        data: bindings::HookBody,
    ) -> anyhow::Result<bindings::HookBody> {
        let (message_tx, message_rx) = oneshot::channel::<bindings::HookBody>();

        let request = HookRequest::new(
            HookRequestInit {
                caller,
                method,
                data,
            },
            message_tx,
        );
        self.hooks_request_tx.send(request).await?;

        match tokio::time::timeout(std::time::Duration::from_secs(5), message_rx).await? {
            Ok(response) => {
                tracing::info!("hook response: {:?}", response);
                Ok(response)
            }
            Err(_) => {
                tracing::info!("hook response timed out");
                anyhow::bail!("failed to receive hook response");
            }
        }
    }
}
