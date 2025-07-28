use std::sync::{atomic::AtomicU64, Arc};

use axum::{
    body::Body,
    extract::{Path, State, WebSocketUpgrade},
    response::Response,
    routing::{get, post},
};
use maf_container::{
    server::{handle_ws_upgrade, ErrorResponse},
    wasi::bindings::{self, HookRequestCaller},
    ContainerRuntime,
};
use schemas::apps::RoomCreationStrategy;

use crate::{
    config::{ProjectConfig, ProjectConfigExt},
    dev::rooms::{DevRoomsStorage, InsertRoom},
    print_dimmed, Context,
};

#[derive(Debug)]
pub struct DevServerConfig {
    pub port: u16,
    pub wasm_module_path: String,
    pub watch: bool,
}

#[derive(Debug, Clone)]
pub struct DevServerState {
    pub project: Option<ProjectConfig>,
    pub rooms: Arc<DevRoomsStorage>,
    pub runtime: Arc<ContainerRuntime>,
}

pub async fn start_local_server(
    context: &mut Context,
    config: DevServerConfig,
) -> anyhow::Result<()> {
    if let Some(project_config) = context.project_config.as_ref() {
        print_dimmed!(
            "[dev] Read project config from {}",
            project_config.base.join("maf-project.toml").display()
        );
    }

    let room_creation_strategy = context.project_config.room_creation_strategy_or_default();

    print_dimmed!(
        "[dev] Using room creation strategy: {}",
        room_creation_strategy.format_with_description()
    );

    match room_creation_strategy {
        RoomCreationStrategy::AuthenticatedApiRequest => {
            print_dimmed!("[dev] - No rooms will be created by default. You must create a room using the API before connecting.");
        }
        RoomCreationStrategy::AutoCreate => {
            print_dimmed!("[dev] - A default room will be created automatically when you connect.");
        }
    }

    // ContainerRuntime uses this variable to track whether the app is active. In dev mode, we
    // use this variable to store but not use the activity, since we don't need to auto stop.
    let app_activity = Box::leak(Box::new(AtomicU64::new(0)));
    let runtime = ContainerRuntime::init(app_activity)?;

    let state = DevServerState {
        project: context.project_config.clone(),
        runtime: Arc::new(runtime),
        rooms: Arc::new(DevRoomsStorage::new(
            &config.wasm_module_path,
            context.project_config.clone(),
        )?),
    };

    // Implement a subset of Platform APIs for the developer server
    let app = axum::Router::new()
        .route(
            "/@/{org_slug}/{app_slug}/{room_id}/connect",
            get(connect_route),
        )
        .route(
            "/@/{org_slug}/{app_slug}/{room_id}/hooks/{method}",
            post(hook_request_handler),
        )
        .with_state(state.clone());

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    println!("[dev] Development server listening on {}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn connect_route(
    State(state): State<DevServerState>,
    Path((_org_slug, _app_name, room_key)): Path<(String, String, String)>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let room = match state.rooms.get_by_key_or_id(&room_key).await {
        Some(room) => room,
        None => {
            // If the room does not exist, automatically create it if the strategy is AutoCreate
            if state.project.room_creation_strategy_or_default() == RoomCreationStrategy::AutoCreate
            {
                state
                    .rooms
                    .insert(
                        &state,
                        InsertRoom {
                            strategy: RoomCreationStrategy::AutoCreate,
                            key: "default".to_string(),
                        },
                    )
                    .await?;

                state
                    .rooms
                    .get_by_key_or_id("default")
                    .await
                    .expect("Default room should exist")
            } else {
                return Err(ErrorResponse::not_found(Some(&format!(
                    "Room with key or ID `{}` not found",
                    room_key
                ))));
            }
        }
    };

    Ok(handle_ws_upgrade(ws, room.inner.clone()).await)
}

async fn hook_request_handler(
    State(state): State<DevServerState>,
    Path((_org_slug, _app_name, room_id, method)): Path<(String, String, String, String)>,
) -> Result<Response, ErrorResponse> {
    let room = state
        .rooms
        .get_by_key_or_id(&room_id)
        .await
        .ok_or_else(|| {
            ErrorResponse::not_found(Some(&format!(
                "Room with key or ID '{}' not found",
                room_id
            )))
        })?;

    // TODO: handle hook bodies
    let response = room
        .inner
        .call_hook(
            HookRequestCaller::Service,
            method.clone(),
            bindings::HookBody::None,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let response = match response {
        bindings::HookBody::None => Response::builder().body(Body::empty())?,
        bindings::HookBody::Json(json) => Response::builder()
            .header("Content-Type", "application/json")
            .body(Body::from(json))?,
    };

    Ok(response)
}
