use axum::ServiceExt;
use axum::serve::ListenerExt;
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

use crate::api::Environment;

mod api;
mod dev_console;
pub mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const DEFAULT_LOG_SETTINGS: &str = "maf_core=info,maf_platform_host=info";
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
    let listener = tokio::net::TcpListener::bind(address)
        .await?
        .tap_io(|tcp_stream| {
            if let Err(err) = tcp_stream.set_nodelay(true) {
                tracing::warn!("failed to set TCP_NODELAY on incoming connection: {err}");
            }
        });

    tracing::info!("Server is listening on {}", address);

    if state.environment() == Environment::Development {
        let dev_console = dev_console::DevConsole::new(state.clone());
        tokio::spawn(async move {
            if let Err(e) = dev_console.run().await {
                tracing::error!("Development console error: {}", e);
            }
        });
    }

    axum::serve(
        listener,
        ServiceBuilder::new()
            // Removes trailing slashes from all request paths. e.g. "/path/" -> "/path"
            .layer(NormalizePathLayer::trim_trailing_slash())
            .service(app.into_service())
            .into_make_service(),
    )
    .with_graceful_shutdown(async move { state.cancel_token().cancelled().await })
    .await?;

    tracing::info!("Good night!");

    Ok(())
}
