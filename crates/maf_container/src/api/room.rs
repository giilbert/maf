use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::{container::Container, storage::bundle::Bundle};

use super::{connection::ConnectionHandle, state::AppState};

#[derive(Debug, Clone)]
pub struct Room {
    pub id: Uuid,
    pub connection_tx: mpsc::Sender<ConnectionHandle>,
}

impl Room {
    pub async fn new(state: &AppState, app_id: Uuid) -> anyhow::Result<(Self, Container)> {
        let bundle = match state.bundle_storage.load_app_bundle(app_id).await? {
            Some(bundle) => bundle,
            None => anyhow::bail!("app bundle not found"),
        };

        Self::new_from_bundle(state, bundle).await
    }

    pub async fn new_test(state: &AppState) -> anyhow::Result<(Self, Container)> {
        Self::new_from_bundle(state, state.bundle_storage.load_test_app().await?).await
    }

    pub async fn add_connection(&self, connection: ConnectionHandle) -> anyhow::Result<()> {
        tracing::info!("adding connection to room {}", self.id);
        self.connection_tx.send(connection).await?;
        Ok(())
    }

    async fn new_from_bundle(
        state: &AppState,
        bundle: Bundle,
    ) -> anyhow::Result<(Self, Container)> {
        tracing::info!("creating new room...");

        let mut container =
            Container::load_from_binary(&state.container_runtime, bundle.wasm_module).await?;

        let mut output = container.take_output().expect("failed to take output");
        let connection_tx = container.store.data().connection_tx.clone();

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
                connection_tx,
            },
            container,
        ))
    }
}
