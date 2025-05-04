// use maf_container::{Connection, Container};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{BoxedConnection, Connection, Container, ContainerRuntime};

use super::Bundle;

#[derive(Debug, Clone)]
pub struct Room {
    pub id: Uuid,
    connection_tx: mpsc::Sender<BoxedConnection>,
}

impl Room {
    pub async fn add_connection(&self, connection: impl Connection) -> anyhow::Result<()> {
        match self.connection_tx.send(Box::new(connection)).await {
            Ok(_) => tracing::info!("connection added to room {}", self.id),
            Err(_) => anyhow::bail!("failed to add connection to room {}", self.id),
        }

        Ok(())
    }

    pub async fn new(
        container: &ContainerRuntime,
        bundle: Bundle,
    ) -> anyhow::Result<(Self, Container)> {
        tracing::info!("creating new room...");

        let mut container = Container::load_from_binary(&container, bundle.wasm_module).await?;

        let mut output = container.take_output().expect("failed to take output");

        tokio::spawn(async move {
            while let Some(line) = output.recv().await {
                tracing::info!(
                    "container: {}",
                    serde_json::to_string(&line).unwrap_or_else(|_| line.clone())
                );
            }
        });

        Ok((
            Self {
                id: Uuid::new_v4(),
                connection_tx: container.store.data().connection_tx.clone(),
            },
            container,
        ))
    }
}
