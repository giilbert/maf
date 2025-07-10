use axum::{
    body::Body,
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
    routing::{get, post},
    Router,
};
use maf_container::{
    server::{handle_ws_upgrade, ErrorResponse, Room},
    wasi::bindings::{self, HookRequestCaller},
};
use schemas::{apps::RoomCreationStrategy, project_config::ProjectConfigFile};
use serde::Deserialize;
use uuid::Uuid;

use crate::storage::{db::app, repos::app_repo};

use super::state::{AppState, Environment};

pub fn create_gateway_router(_state: AppState) -> Router<AppState> {
    let inner = Router::new()
        .route("/{room_id}/connect", get(connect_route))
        .route("/{room_id}/hooks/{method}", post(hook_request_handler));

    Router::new().nest("/@/{org_slug}/{app_name}", inner)
}

#[derive(Deserialize)]
pub struct ConnectQueryParams {
    secret: Option<String>,
}

async fn connect_route(
    State(state): State<AppState>,
    Path((org_slug, app_name, room_id)): Path<(String, String, String)>,
    Query(query_params): Query<ConnectQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let app = app_repo::get_app_by_name_and_org_slug(&state.db, &app_name, &org_slug)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let room_creation_strategy = match app.as_ref().map(|app| app.config.clone()).flatten() {
        Some(config) => {
            let parsed_config = toml::from_str::<ProjectConfigFile>(&config).map_err(|_| {
                ErrorResponse::bad_request(Some(&format!("failed to parse app config")))
            })?;

            parsed_config.rooms
        }
        None => {
            if state.environment == Environment::Development {
                RoomCreationStrategy::AutoCreate
            } else {
                RoomCreationStrategy::AuthenticatedApiRequest
            }
        }
    };

    let room = match room_creation_strategy {
        RoomCreationStrategy::AuthenticatedApiRequest => {
            let room = get_api_created_room(&state, room_id).await?;

            if let Some(secret) = query_params.secret {
                if secret != room.room_secret {
                    return Err(ErrorResponse::forbidden(Some(
                        "Invalid room secret provided.",
                    )));
                }
            } else {
                return Err(ErrorResponse::bad_request(Some(
                    "Room secret is required for api-created rooms.",
                )));
            }

            room
        }
        RoomCreationStrategy::AutoCreate => {
            if room_id != "default" {
                return Err(ErrorResponse::bad_request(Some(
                    "Only `default` room is supported for autocreated rooms.",
                )));
            }

            fetch_autocreated_room(app, &state, org_slug).await?
        }
    };

    Ok(handle_ws_upgrade(ws, room).await)
}

async fn hook_request_handler(
    State(state): State<AppState>,
    Path((org_slug, _app_name, room_id, method)): Path<(String, String, String, String)>,
) -> Result<Response, ErrorResponse> {
    if room_id != "default" {
        return Err(ErrorResponse::forbidden(Some(
            "only autocreated rooms are supported right now",
        )));
    }

    // TODO: handle other room types other than autocreated
    let room_id = *state
        .auto_created_rooms_by_org_slug
        .read()
        .await
        .get(&org_slug)
        .ok_or_else(|| ErrorResponse::not_found(Some("app not found")))?;

    let room = state
        .rooms
        .read()
        .await
        .get(&room_id)
        .expect("room not found")
        .clone();

    // TODO: handle hook bodies
    let response = room
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

async fn fetch_autocreated_room(
    app: Option<app::Model>,
    state: &AppState,
    org_slug: String,
) -> Result<Room, ErrorResponse> {
    // The code is structured this way to avoid deadlocks
    let existing_room_id = state
        .auto_created_rooms_by_org_slug
        .read()
        .await
        .get(&org_slug)
        .cloned();

    let room_id = match existing_room_id {
        Some(id) => id,
        None => {
            let (room, mut container) = match app {
                Some(app) => {
                    Room::new(
                        &state.container_runtime,
                        state
                            .bundle_storage
                            .load_app_bundle(app.id)
                            .await?
                            .ok_or_else(|| {
                                ErrorResponse::not_found(Some("app bundle not found"))
                            })?,
                    )
                    .await?
                }
                None if state.environment == Environment::Development => {
                    tracing::info!("App not found. Defaulting to test app (development only)");
                    Room::new(
                        &state.container_runtime,
                        state.bundle_storage.load_test_app().await?,
                    )
                    .await?
                }
                None => return Err(ErrorResponse::not_found(Some("app not found"))),
            };

            let room_id = room.id;
            state
                .auto_created_rooms_by_org_slug
                .write()
                .await
                .insert(org_slug.clone(), room.id);

            state.rooms.write().await.insert(room_id, room);

            let state = state.clone();
            container.pass_output();
            container.start_inactive_shutdown_task();

            tokio::spawn(async move {
                if let Err(e) = container.run().await {
                    tracing::error!("container {} error: {e:?}", container.id);
                }
                tracing::info!("container {} stopped", container.id);

                state
                    .auto_created_rooms_by_org_slug
                    .write()
                    .await
                    .remove(&org_slug)
                    .unwrap_or_default();

                state.rooms.write().await.remove(&room_id);
            });

            room_id
        }
    };

    Ok(state
        .rooms
        .read()
        .await
        .get(&room_id)
        .expect("room not found")
        .clone())
}

async fn get_api_created_room(state: &AppState, room_id: String) -> Result<Room, ErrorResponse> {
    let room_id = Uuid::parse_str(&room_id)
        .map_err(|_| ErrorResponse::bad_request(Some("invalid room id format")))?;

    state
        .rooms
        .read()
        .await
        .get(&room_id)
        .cloned()
        .ok_or_else(|| ErrorResponse::not_found(Some("room not found")))
}
