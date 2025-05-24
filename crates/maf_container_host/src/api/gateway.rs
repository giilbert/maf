use axum::{
    body::Body,
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
    routing::{get, post},
    Router,
};
use maf_container::{
    server::{handle_ws_upgrade, Room},
    wasi::bindings::{self, HookRequestCaller},
};
use serde::Deserialize;

use crate::{api::ErrorResponse, storage::repos::app_repo};

use super::{
    state::{AppState, Environment},
    user_app::RoomCreationStrategy,
};

pub fn create_gateway_router(_state: AppState) -> Router<AppState> {
    let inner = Router::new()
        .route("/connect", get(connect_route))
        .route("/{room_id}/hooks/{method}", post(hook_request_handler));

    Router::new().nest("/@/{org_slug}/{app_slug}", inner)
}

#[derive(Deserialize)]
pub struct ConnectQueryParams {}

async fn connect_route(
    State(state): State<AppState>,
    Path((org_slug, app_name)): Path<(String, String)>,
    Query(_query_params): Query<ConnectQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    // TODO: replace logic with actual user app query from the database

    let app = app_repo::get_app_by_name_and_org_slug(&state.db, &app_name, &org_slug)
        .await
        .map_err(|e| anyhow::anyhow!(e))?
        .ok_or_else(|| ErrorResponse::not_found(Some("app not found")));

    let room_creation_strategy = RoomCreationStrategy::AutoCreate; // TODO: get from app

    let room = match room_creation_strategy {
        RoomCreationStrategy::AutoCreate => {
            // The code is structured this way to avoid deadlocks
            let room_id = state
                .auto_created_rooms_by_org_slug
                .read()
                .await
                .get(&org_slug)
                .map(|room_id| room_id.clone());

            let room_id = match room_id {
                Some(room_id) => room_id,
                None => {
                    let (room, mut container) = match app {
                        Ok(app) => {
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
                        Err(_) if state.environment == Environment::Development => {
                            tracing::info!(
                                "App not found. Defaulting to test app (development only)"
                            );
                            Room::new(
                                &state.container_runtime,
                                state.bundle_storage.load_test_app().await?,
                            )
                            .await?
                        }
                        Err(e) => return Err(e),
                    };

                    let mut output = container.take_output().expect("failed to take output");

                    tokio::spawn(async move {
                        while let Some(line) = output.recv().await {
                            tracing::info!(
                                "container: {}",
                                serde_json::to_string(&line).unwrap_or_else(|_| line.clone())
                            );
                        }
                    });

                    let room_id = room.id;
                    state
                        .auto_created_rooms_by_org_slug
                        .write()
                        .await
                        .insert(org_slug.clone(), room.id);

                    state.rooms.write().await.insert(room_id, room);

                    let state = state.clone();
                    tokio::spawn(async move {
                        container.start_inactive_shutdown_task();

                        if let Err(e) = container.run().await {
                            tracing::error!("container error: {e:?}");
                        }
                        tracing::info!("container stopped");

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

            state
                .rooms
                .read()
                .await
                .get(&room_id)
                .expect("room not found")
                .clone()
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

    let response = room
        .call_hook(
            HookRequestCaller::Service,
            method.clone(),
            bindings::HookBody::None,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let response = match response {
        bindings::HookBody::None => Response::builder()
            .body(Body::empty())
            .expect("failed to build response"),
        bindings::HookBody::Json(json) => Response::builder()
            .header("Content-Type", "application/json")
            .body(Body::from(json))
            .expect("failed to build response"),
    };

    Ok(response)
}
