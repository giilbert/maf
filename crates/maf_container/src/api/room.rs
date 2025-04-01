use uuid::Uuid;

use crate::container::Container;

use super::state::AppState;

#[derive(Debug)]
pub struct Room {
    pub id: Uuid,
    pub container: Container,
}

impl Room {
    pub async fn new(state: &AppState) -> anyhow::Result<Self> {
        tracing::info!("creating new room...");

        let mut container = Container::load_from_file(
            &state.container_runtime,
            "target/debug/wasm32-wasip2/example_basic.wasm",
        )
        .await?;

        let mut output = container.take_output().expect("failed to take output");

        tokio::spawn(async move {
            while let Some(line) = output.recv().await {
                tracing::info!("out: {}", line);
            }
        });

        Ok(Self {
            id: Uuid::new_v4(),
            container,
        })
    }
}
