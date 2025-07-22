use axum::{
    body::Body,
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
    routing::{get, post},
    Router,
};
use maf_container::{
    server::{handle_ws_upgrade, ErrorResponse, RoomInner},
    wasi::bindings::{self, HookRequestCaller},
};
use schemas::{apps::RoomCreationStrategy, project_config::ProjectConfigFile};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::rooms::{AppNameAndOrgSlug, InsertRoom, Room},
    storage::{db::app, repos::app_repo},
};

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
    let app = app_repo::get_app_by_name_and_org_slug(&state.db, &app_name, &org_slug).await?;

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
            let room = state
                .rooms
                .get(
                    &Uuid::parse_str(&room_id)
                        .map_err(|_| ErrorResponse::bad_request(Some("Invalid room ID format")))?,
                )
                .await
                .ok_or_else(|| {
                    ErrorResponse::not_found(Some("Room not found or not created via API"))
                })?;

            if let Some(secret) = query_params.secret {
                if secret != room.meta.secret {
                    return Err(ErrorResponse::forbidden(Some(
                        "Invalid room secret provided.",
                    )));
                }
            } else {
                return Err(ErrorResponse::bad_request(Some(
                    "Room secret is required for api-created rooms.",
                )));
            }

            room.clone()
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

    Ok(handle_ws_upgrade(ws, room.inner).await)
}

async fn hook_request_handler(
    State(state): State<AppState>,
    Path((org_slug, app_name, room_id, method)): Path<(String, String, String, String)>,
) -> Result<Response, ErrorResponse> {
    if room_id != "default" {
        return Err(ErrorResponse::forbidden(Some(
            "only autocreated rooms are supported right now",
        )));
    }

    // TODO: handle other room types other than autocreated
    let room = &state
        .rooms
        .get(
            &Uuid::parse_str(&room_id)
                .map_err(|_| ErrorResponse::bad_request(Some("Invalid room ID format")))?,
        )
        .await
        .ok_or_else(|| ErrorResponse::not_found(Some("app not found")))?;

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

async fn fetch_autocreated_room(
    app: Option<app::Model>,
    state: &AppState,
    org_slug: String,
) -> Result<Room, ErrorResponse> {
    // The code is structured this way to avoid deadlocks
    let existing_room_id = state
        .rooms
        .auto_created_rooms
        .read()
        .await
        .get(&AppNameAndOrgSlug {
            app: app
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or("development".to_string()),
            org: org_slug.clone(),
        })
        .cloned();

    let room_id = match existing_room_id {
        Some(id) => id,
        None => {
            let (room, mut container) = match &app {
                Some(app) => {
                    RoomInner::new(
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
                    RoomInner::new(
                        &state.container_runtime,
                        state.bundle_storage.load_test_app().await?,
                    )
                    .await?
                }
                None => return Err(ErrorResponse::not_found(Some("app not found"))),
            };

            // In development, if the app is not found, we use the test app
            let app_name = app
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or("development".to_string());

            let app_name_clone = app_name.clone();
            let room_id = room.id();
            state
                .rooms
                .insert(InsertRoom {
                    room,
                    strategy: RoomCreationStrategy::AutoCreate,
                    app: AppNameAndOrgSlug {
                        app: app_name_clone,
                        org: org_slug.clone(),
                    },
                })
                .await;

            let state = state.clone();
            container.pass_output();
            container.start_inactive_shutdown_task();

            tokio::spawn(async move {
                if let Err(e) = container.run().await {
                    tracing::error!("container {} error: {e:?}", container.id);
                }
                tracing::info!("container {} stopped", container.id);

                state.rooms.remove(&room_id).await;
            });

            room_id
        }
    };

    Ok(state
        .rooms
        .get(&room_id)
        .await
        .expect("room not found")
        .clone())
}
