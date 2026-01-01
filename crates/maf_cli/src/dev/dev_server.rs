//! An implementation of a development server for running and testing MAF applications locally.
//!
//! This is simplified version of the MAF Platform server, intended for use during development. It
//! supports creating rooms, connecting via WebSocket, and handling hook requests. See
//! `maf_container_host::api::gateway` for the full implementation of the API routes.

use std::sync::{atomic::AtomicU64, Arc};

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
    routing::get,
    ServiceExt,
};
use colored::Colorize;
use maf_container::{
    server::{get_auth_data, handle_ws_upgrade, pre_create_room_auth_check},
    Container, ContainerResourceLimit, ContainerRuntime, CreateContainerOptions,
};
use maf_schemas::{
    apps::{ConnectQueryParams, InfoResponse, MetaVisibility, RoomCreationStrategy},
    error::ErrorResponse,
};
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;
use uuid::Uuid;

use crate::{
    config::{ProjectConfig, ProjectConfigExt},
    dev::{
        platform::create_platform_api_router,
        rooms::{DevRoomsStorage, InsertRoom},
        typed,
    },
    print_dimmed, Context,
};

#[derive(Debug)]
pub struct DevServerConfig {
    pub port: u16,
    pub wasm_module_path: String,
    pub _watch: bool,
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
        "[dev] - Using room creation strategy: {}",
        room_creation_strategy.format_with_description()
    );
    if let Some(auth_mode) = context
        .project_config
        .as_ref()
        .and_then(|p| p.data.auth.clone())
        .map(|a| a.mode)
    {
        print_dimmed!(
            "[dev] - Authentication mode: {}",
            auth_mode.format_with_description()
        );
    }

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

    // Generate types if the project config is set to do so
    if let Some(project) = state.project.clone() {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = generate_types(state, project).await {
                println!("{}", format!("[dev] Failed to generate types: {e}").red());
            }
        });
    }

    let gateway_router = axum::Router::new()
        .route("/", get(info_route))
        .route("/connect", get(connect_route));
    // .route("/hooks/{method}", post(hook_request_handler));

    // Implement a subset of Platform APIs for the developer server
    let app = axum::Router::new()
        .nest("/@/{org_slug}/{app_name}/{room_key}", gateway_router)
        .merge(create_platform_api_router())
        .with_state(state.clone());

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    println!("[dev] Development server listening on {}", address);
    axum::serve(
        listener,
        ServiceBuilder::new()
            // Removes trailing slashes from all request paths. e.g. "/path/" -> "/path"
            .layer(NormalizePathLayer::trim_trailing_slash())
            .service(app.into_service())
            .into_make_service(),
    )
    .await?;

    Ok(())
}

async fn generate_types(state: DevServerState, project: ProjectConfig) -> anyhow::Result<()> {
    let mut container = Container::load_from_binary(
        &state.runtime,
        Uuid::nil(),
        CreateContainerOptions {
            bytes: &state.rooms.bundle.wasm_module_bytes,
            resource_limit: ContainerResourceLimit::small_defaults(),
            meta: None,
            secret: "".to_string(),
        },
    )
    .await?;

    container.dry_run().await?;

    let schema = container.recv_app_schema().await?;
    tracing::debug!("{}", format!("App schema received: {schema:?}").dimmed());

    typed::create_types_file_for_project(&project, schema).await?;

    Ok(())
}

async fn info_route(
    State(state): State<DevServerState>,
    Path((_org_slug, _app_name, room_key)): Path<(String, String, String)>,
) -> Result<axum::Json<InfoResponse>, ErrorResponse> {
    let meta = state
        .rooms
        .get_by_key_or_id(&room_key)
        .await
        .ok_or_else(|| ErrorResponse::not_found(Some("room not found")))?
        .inner
        .container
        .meta
        .list_values::<std::collections::BTreeMap<String, serde_json::Value>>(
            MetaVisibility::Public,
        )
        .await;

    Ok(axum::Json(InfoResponse { meta }))
}

async fn connect_route(
    State(state): State<DevServerState>,
    Path((_org_slug, _app_name, room_key)): Path<(String, String, String)>,
    Query(query_params): Query<ConnectQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let auth_mode = state
        .project
        .as_ref()
        .cloned()
        .and_then(|p| p.data.auth)
        .map(|a| a.mode);

    let _ = pre_create_room_auth_check(&query_params, auth_mode.as_ref())?;
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
                            key: Some("default".to_string()),
                            // AutoCreate rooms do not have meta initially
                            meta: None,
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
    let auth_data = get_auth_data(&query_params, auth_mode.as_ref(), &room.inner)?;

    Ok(handle_ws_upgrade(ws, room.inner.clone(), auth_data).await)
}

// async fn hook_request_handler(
//     State(state): State<DevServerState>,
//     Path((_org_slug, _app_name, room_id, method)): Path<(String, String, String, String)>,
// ) -> Result<Response, ErrorResponse> {
//     let room = state
//         .rooms
//         .get_by_key_or_id(&room_id)
//         .await
//         .ok_or_else(|| {
//             ErrorResponse::not_found(Some(&format!(
//                 "Room with key or ID '{}' not found",
//                 room_id
//             )))
//         })?;

//     // TODO: handle hook bodies
//     let response = room
//         .inner
//         .call_hook(
//             HookRequestCaller::Service,
//             method.clone(),
//             bindings::HookBody::None,
//         )
//         .await
//         .map_err(|e| anyhow::anyhow!(e))?;

//     let response = match response {
//         bindings::HookBody::None => Response::builder().body(Body::empty())?,
//         bindings::HookBody::Json(json) => Response::builder()
//             .header("Content-Type", "application/json")
//             .body(Body::from(json))?,
//     };

//     Ok(response)
// }
