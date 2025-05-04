use std::sync::Arc;

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::{ErrorResponse, Response},
    routing::get,
};
use maf_container::server::{handle_ws_upgrade, Room};
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
    container_runtime: maf_container::ContainerRuntime,
}

pub async fn start_dev_server(config: DevServerConfig) -> anyhow::Result<()> {
    let address = format!("0.0.0.0:{}", config.port);
    pretty::info!("starting server on {}...", address);

    let room = load_room(&config.wasm_module_path).await?;

    let state = DevServerState {
        inner: Arc::new(StateInner {
            room: RwLock::new(room),
            container_runtime: maf_container::ContainerRuntime::init()?,
        }),
    };

    let app = axum::Router::new()
        .route("@/{org_slug}/{app_slug}", get(connect_route))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(address).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn load_room(path: &str) -> anyhow::Result<Room> {
    todo!();
}

async fn connect_route(
    State(state): State<DevServerState>,
    Path((org_slug, app_name)): Path<(String, String)>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let room = state.inner.room.read().await.clone();
    Ok(handle_ws_upgrade(ws, room).await)
}
