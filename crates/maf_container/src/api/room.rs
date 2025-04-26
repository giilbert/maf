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
    pub async fn new(state: &AppState, app_id: Uuid) -> anyhow::Result<Self> {
        tracing::info!("creating new room...");

        let bundle = match state.bundle_storage.load_app_bundle(app_id).await? {
            Some(bundle) => bundle,
            None => {
                if dotenvy::var("ENVIRONMENT").unwrap_or_default() == "development" {
                    tracing::info!("app bundle not found. loading test app...");
                    state.bundle_storage.load_test_app().await?
                } else {
                    return Err(anyhow::anyhow!("app bundle not found"));
                }
            }
        };

        tracing::info!(
            "first ten bytes of wasm module: {:?}",
            &bundle.wasm_module[..10]
        );

        let mut container =
            Container::load_from_binary(&state.container_runtime, bundle.wasm_module).await?;

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
