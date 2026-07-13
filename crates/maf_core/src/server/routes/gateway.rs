use std::collections::BTreeMap;

use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use maf_schemas::ErrorResponse;
use maf_schemas::apps::{ConnectQueryParams, InfoResponse, MetaVisibility, RoomCreationStrategy};

use crate::server::RoomHostImpl;
use crate::server::routes::types::AppRoomPath;

const ERR_ROOM_NOT_FOUND: &str = "room not found";

/// Create the router for the MAF Platform API routes that are used by clients for a particular app
/// and/or room. This includes the WebSocket connection route.
pub fn create_gateway_router<R: RoomHostImpl>() -> Router<R> {
    Router::<R>::new().route(
        "/@/{org_slug}/{app_name}/{room_key}",
        get(get_room_info_route::<R>),
    )
}

/// GET `/@/{org_slug}/{app_name}/{room_key}`
///
/// Returns public meta information about the specified room, or a 404 error if the room does not
/// exist.
///
/// TODO: "room visiblity" option for rooms that exist, but do not want to be publicly discoverable.
async fn get_room_info_route<R: RoomHostImpl>(
    State(host): State<R>,
    Path(path): Path<AppRoomPath>,
) -> Result<Json<InfoResponse>, ErrorResponse> {
    let room = host
        .room_storage()
        .get_by_key(&path.app_org(), &path.room_key)
        .await
        .ok_or_else(|| ErrorResponse::not_found(Some(ERR_ROOM_NOT_FOUND)))?;

    let meta = room
        .meta_storage()
        .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Public)
        .await;

    Ok(Json(InfoResponse { meta }))
}

/// GET `/@/{org_slug}/{app_name}/{room_key}/connect`
///
/// Handles WebSocket connection requests to a room. This route is used by MAF clients to subscribe
/// to realtime events from a room.
///
/// FIXME: There is no way for clients to get an error message if something goes wrong here since
/// it is a WebSocket upgrade request. Consider adding a preliminary HTTP request to validate
/// parameters before attempting the upgrade.
async fn connect_route<R: RoomHostImpl>(
    State(host): State<R>,
    Path(path): Path<AppRoomPath>,
    Query(query_params): Query<ConnectQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let app = host
        .app(path.app_org())? // Handle internal server errors when fetching the app.
        // XXX: users can't actually use this error message since this is a WebSocket upgrade
        .ok_or_else(|| ErrorResponse::not_found(Some("app not found")))?;

    let room_creation_strategy = app.config().rooms;

    let room = host.room_storage().get_by_key(&app, &path.room_key).await;

    // We need to start the room if it exists, or create it if it doesn't exist and the strategy
    // allows for it.
    if let RoomCreationStrategy::AutoCreate = room_creation_strategy {
        // Create the room automatically since it doesn't exist and the strategy allows for it.
        let _ = host
            .room_storage()
            .insert(
                &host,
                crate::server::room_storage::InsertRoom {
                    strategy: RoomCreationStrategy::AutoCreate,
                    key: Some(path.room_key.clone()),
                    meta: None,
                },
            )
            .await?;
    } else if room.is_none() {
        // The room doesn't exist and the strategy doesn't allow for automatic creation.
        return Err(ErrorResponse::not_found(Some(ERR_ROOM_NOT_FOUND)));
    };

    todo!();
}
