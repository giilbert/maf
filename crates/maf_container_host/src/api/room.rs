use maf_container::{Connection, Container};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::storage::bundle::Bundle;

use super::{connection::ConnectionHandle, state::AppState};

#[derive(Debug, Clone)]
pub struct Room {
    pub id: Uuid,
    connection_tx: mpsc::Sender<Box<dyn Connection>>,
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
        match self.connection_tx.send(Box::new(connection)).await {
            Ok(_) => tracing::info!("connection added to room {}", self.id),
            Err(_) => anyhow::bail!("failed to add connection to room {}", self.id),
        }

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
