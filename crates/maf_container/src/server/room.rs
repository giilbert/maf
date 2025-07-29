use schemas::apps::RoomId;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    Connection, Container, ContainerRuntime,
    container::{ContainerHandle, ContainerResourceLimit},
    wasi::{
        HookRequest,
        bindings::{self, HookRequestCaller, HookRequestInit},
    },
};

use super::Bundle;

#[derive(Debug, Clone)]
pub struct RoomInner {
    pub container: ContainerHandle,
    id: Uuid,
}

impl RoomInner {
    pub async fn new(
        container_runtime: &ContainerRuntime,
        bundle: Bundle,
        resource_limit: ContainerResourceLimit,
    ) -> anyhow::Result<(Self, Container)> {
        let room_id = Uuid::new_v4();
        let container = Container::load_from_binary(
            &container_runtime,
            bundle.wasm_module,
            room_id,
            resource_limit,
        )
        .await?;

        Ok((
            Self {
                id: room_id,
                container: container.handle(),
            },
            container,
        ))
    }

    pub async fn replace_container(&mut self, container: Container) -> anyhow::Result<()> {
        self.container = container.handle();
        Ok(())
    }

    /// Returns the unique identifier of the room.
    pub fn id(&self) -> RoomId {
        self.id
    }

    pub async fn add_connection(&self, connection: impl Connection) -> anyhow::Result<()> {
        match self.container.add_connection(Box::new(connection)).await {
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
        self.container.send_hook_request(request).await?;

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
