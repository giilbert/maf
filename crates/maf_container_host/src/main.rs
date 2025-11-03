use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::api::Environment;

mod api;
mod dev_console;
pub mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const DEFAULT_LOG_SETTINGS: &str = "maf_container=info,maf_container_host=info";
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .parse(std::env::var("RUST_LOG").unwrap_or(DEFAULT_LOG_SETTINGS.to_string()))?,
        )
        .init();

    match dotenvy::dotenv() {
        Ok(_) => tracing::info!("Loaded environment variables from .env file"),
        Err(e) => tracing::warn!("Failed to load .env file: {}", e),
    }

    if let Err(e) = try_main().await {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn try_main() -> anyhow::Result<()> {
    let address = "0.0.0.0:1147";

    tracing::info!("Initializing server...");
    let (state, app) = api::create_app().await?;
    let listener = tokio::net::TcpListener::bind(address).await?;

    let state_clone = state.clone();
    tracing::info!("Server is listening on {}", address);

    if state.environment == Environment::Development {
        let dev_console = dev_console::DevConsole::new(state.clone());
        tokio::spawn(async move {
            if let Err(e) = dev_console.run().await {
                tracing::error!("Development console error: {}", e);
            }
        });
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { state_clone.cancel_server.cancelled().await })
        .await?;

    tracing::info!("Good night!");

    Ok(())
}
