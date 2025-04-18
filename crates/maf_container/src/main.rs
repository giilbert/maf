use runtime::ContainerRuntime;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod api;
mod container;
mod runtime;
pub mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    dotenvy::dotenv()?;

    if let Err(e) = try_main().await {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn try_main() -> anyhow::Result<()> {
    let address = "0.0.0.0:3000";

    let (_state, app) = api::create_app().await?;
    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!("starting server on {}", address);
    axum::serve(listener, app).await?;

    Ok(())
}
