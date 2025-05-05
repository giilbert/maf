use std::sync::Arc;

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::{ErrorResponse, Response},
    routing::get,
};
use colored::Colorize;
use maf_container::{
    server::{handle_ws_upgrade, Bundle, Room},
    ContainerRuntime,
};
use tokio::sync::RwLock;

use crate::pretty;

#[derive(Debug)]
pub struct DevServerConfig {
    pub port: u16,
    pub wasm_module_path: String,
}

#[derive(Debug, Clone)]
struct DevServerState {
    inner: Arc<StateInner>,
}

#[derive(Debug)]
struct StateInner {
    room: RwLock<Room>,
    container_runtime: ContainerRuntime,
}

pub async fn start_dev_server(config: DevServerConfig) -> anyhow::Result<()> {
    let address = format!("0.0.0.0:{}", config.port);
    pretty::info!("starting maf dev server...");

    let runtime = ContainerRuntime::init()?;
    let room = load_room(&runtime, &config.wasm_module_path).await?;

    let state = DevServerState {
        inner: Arc::new(StateInner {
            room: RwLock::new(room),
            container_runtime: runtime,
        }),
    };

    let app = axum::Router::new()
        .route("/@/{org_slug}/{app_slug}/connect", get(connect_route))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(address).await?;

    pretty::info!("dev server listening on {}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn load_room(runtime: &ContainerRuntime, path: &str) -> anyhow::Result<Room> {
    let bytes = tokio::fs::read(path).await?;

    let bundle = Bundle {
        wasm_module: bytes.into(),
    };

    let (room, mut container) = Room::new(&runtime, bundle).await?;

    let mut output = container.take_output().expect("failed to take output");
    tokio::spawn(async move {
        while let Some(line) = output.recv().await {
            println!(
                "{} {}",
                ">".black().on_blue(),
                serde_json::to_string(&line).unwrap_or_else(|_| line.clone())
            );
        }
    });

    tokio::spawn(async move {
        if let Err(e) = container.run().await {
            pretty::error!("failed to run container: {}", e);
        } else {
            pretty::info!("container exited");
        }
    });

    pretty::info!("loaded room from {}", path);

    Ok(room)
}

async fn connect_route(
    State(state): State<DevServerState>,
    Path((org_slug, app_name)): Path<(String, String)>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let room = state.inner.room.read().await.clone();
    Ok(handle_ws_upgrade(ws, room).await)
}
