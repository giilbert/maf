use runtime::ContainerRuntime;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::Directive, fmt, prelude::*, EnvFilter};

mod api;
mod container;
mod runtime;
pub mod storage;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const DEFAULT_LOG_SETTINGS: &str = "maf_container=info";
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
    let address = "0.0.0.0:3000";

    let (state, app) = api::create_app().await?;
    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!("starting server on {}", address);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move { state.cancel_server.cancelled().await })
        .await?;

    tracing::info!("Good night!");

    Ok(())
}
