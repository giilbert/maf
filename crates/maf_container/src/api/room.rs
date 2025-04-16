use tokio::sync::mpsc;
use uuid::Uuid;

use crate::container::Container;

use super::{connection::ConnectionHandle, state::AppState};

#[derive(Debug, Clone)]
pub struct Room {
    pub id: Uuid,
    // pub container: ContainerHan,
    pub connection_tx: mpsc::Sender<ConnectionHandle>,
}

impl Room {
    pub async fn new(state: &AppState) -> anyhow::Result<Self> {
        tracing::info!("creating new room...");

        let mut container = Container::load_from_binary(
            &state.container_runtime,
            state.bundle_storage.load_test_app().await?.wasm_module,
        )
        .await?;

        let mut output = container.take_output().expect("failed to take output");
        let connection_tx = container.store.data().connection_tx.clone();

        tokio::spawn(async move {
            tracing::info!("waiting for stdout...");
            while let Some(line) = output.recv().await {
                tracing::info!(
                    "out: {}",
                    serde_json::to_string(&line).unwrap_or_else(|_| line.clone())
                );
            }
            tracing::info!("stdout done!");
        });

        tokio::spawn(async move {
            if let Err(err) = container.run().await {
                tracing::error!("failed to run container: {err:?}");
            } else {
                tracing::warn!("container finished running. it's dead.");
            }
        });

        Ok(Self {
            id: Uuid::new_v4(),
            connection_tx,
            // container: Arc::new(container),
        })
    }

    pub async fn add_connection(&self, connection: ConnectionHandle) -> anyhow::Result<()> {
        tracing::info!("adding connection to room {}", self.id);
        self.connection_tx.send(connection).await?;
        Ok(())
    }
}
