//! Entrypoints for WebSocket and HTTP hook requests to apps.
//!
//! This module defines the API routes for connecting to app rooms via WebSockets and handling hook
//! requests. It is responsible for routing incoming requests to appropriate rooms and managing room
//! creation strategies based on app configurations.
//!
//! **Routes:**
//! - `GET /@/{org_slug}/{app_name}/{room_key}`:
//!   Retrieves information about a specific room.
//! - `GET /@/{org_slug}/{app_name}/{room_key}/connect`:
//!   Establishes a WebSocket connection to the specified room.
//! - `POST /@/{org_slug}/{app_name}/{room_key}/hooks/{method}`:
//!   Handles hook requests for the specified room.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use maf_core::server::{
    WsUpgradeOptions, do_ws_upgrade, get_auth_data, pre_create_room_auth_check,
};
use maf_schemas::apps::{
    AppNameAndOrgSlug, ConnectQueryParams, InfoResponse, MetaVisibility, RoomCreationStrategy,
};
use maf_schemas::error::ErrorResponse;
use maf_schemas::project_config::ProjectConfigFile;

use super::state::{AppState, Environment};
use crate::storage::repos::app_repo;

pub fn create_gateway_router(_state: AppState) -> Router<AppState> {
    let inner = Router::new()
        .route("/", get(info_route))
        .route("/connect", get(connect_route));
    // .route("/hooks/{method}", post(hook_request_handler));

    Router::new().nest("/@/{org_slug}/{app_name}/{room_key}", inner)
}

/// **GET** `/@/{org_slug}/{app_name}/{room_key}`
/// Returns public meta information about the specified room, or a 404 error if the room does not
/// exist.
async fn info_route(
    State(state): State<AppState>,
    Path((org_slug, app_name, room_key)): Path<(String, String, String)>,
) -> Result<Json<InfoResponse>, ErrorResponse> {
    let meta = state
        .rooms
        .get_by_key_or_id(
            &AppNameAndOrgSlug {
                app: app_name,
                org: org_slug,
            },
            &room_key,
        )
        .await
        .ok_or_else(|| ErrorResponse::not_found(Some("room not found")))?
        .inner()
        .meta_storage()
        .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Public)
        .await;

    Ok(Json(InfoResponse { meta }))
}

/// **GET** `/@/{org_slug}/{app_name}/{room_key}/connect`
///
/// FIXME: There is no way for clients to get an error message if something goes wrong here since
/// it is a WebSocket upgrade request. Consider adding a preliminary HTTP request to validate
/// parameters before attempting the upgrade.
async fn connect_route(
    State(state): State<AppState>,
    Path((org_slug, app_name, room_key)): Path<(String, String, String)>,
    Query(query_params): Query<ConnectQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    // Fetch the app and determine its room creation strategy, defaulting based on environment and
    // whether it is set
    let app = app_repo::get_app_by_name_and_org_slug(&state.db, &app_name, &org_slug).await?;
    let (room_creation_strategy, auth_mode) = match app.as_ref().and_then(|app| app.config.clone())
    {
        Some(config) => {
            let parsed_config = toml::from_str::<ProjectConfigFile>(&config)
                .map_err(|_| ErrorResponse::bad_request(Some("failed to parse app config")))?;

            (parsed_config.rooms, parsed_config.auth.map(|a| a.mode))
        }
        None => (
            if state.environment == Environment::Development {
                RoomCreationStrategy::AutoCreate
            } else {
                RoomCreationStrategy::AuthenticatedApiRequest
            },
            None,
        ),
    };

    pre_create_room_auth_check(&query_params, auth_mode.as_ref())?;

    let room = match room_creation_strategy {
        RoomCreationStrategy::AuthenticatedApiRequest => {
            let room = state
                .rooms
                .get_by_key_or_id(
                    &AppNameAndOrgSlug {
                        app: app_name,
                        org: org_slug,
                    },
                    &room_key,
                )
                .await
                .ok_or_else(|| {
                    ErrorResponse::not_found(Some(&format!(
                        "Room with key or ID `{}` not found",
                        room_key
                    )))
                })?;

            room.clone()
        }
        RoomCreationStrategy::AutoCreate => {
            if room_key != "default" {
                return Err(ErrorResponse::bad_request(Some(
                    "Only `default` room is supported for autocreated rooms.",
                )));
            }

            state
                .rooms
                .fetch_autocreated_room(&state, app, org_slug)
                .await?
        }
    };

    let auth_data = get_auth_data(&query_params, auth_mode.as_ref(), &room.inner())?;

    Ok(do_ws_upgrade(WsUpgradeOptions {
        ws,
        room: room.inner().clone(),
        auth_data,
    })
    .await)
}

// **POST** `/@/{org_slug}/{app_name}/{room_key}/hooks/{method}`
// async fn hook_request_handler(
//     State(state): State<AppState>,
//     Path((org_slug, app_name, room_key, method)): Path<(String, String, String, String)>,
// ) -> Result<Response, ErrorResponse> {
//     if room_key != "default" {
//         return Err(ErrorResponse::forbidden(Some(
//             "only autocreated rooms are supported right now",
//         )));
//     }

//     // TODO: handle other room types other than autocreated
//     let room = &state
//         .rooms
//         .get_by_key_or_id(
//             &AppNameAndOrgSlug {
//                 app: app_name,
//                 org: org_slug,
//             },
//             &room_key,
//         )
//         .await
//         .ok_or_else(|| ErrorResponse::not_found(Some("app not found")))?;

//     // TODO: handle hook bodies
//     let response = room
//         .inner
//         .call_hook(
//             bindings::HookRequestCaller::Service,
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
