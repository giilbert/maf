use std::collections::BTreeMap;

use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use maf_schemas::ErrorResponse;
use maf_schemas::apps::{ConnectQueryParams, MetaVisibility, PublicRoomInfo, RoomCreationStrategy};

use crate::server::app::App;
use crate::server::room_storage::CreateRoomOptions;
use crate::server::types::AppRoomPath;
use crate::server::{
    RoomHostImpl, WsUpgradeOptions, do_ws_upgrade, get_auth_data, pre_create_room_auth_check,
};

/// Create the router for the MAF Platform API routes that are used by clients for a particular app
/// and/or room. This includes the WebSocket connection route.
pub fn create_gateway_router<R: RoomHostImpl>(_host: &R) -> Router<R> {
    // Mounted at /@/{org_slug}/{app_name}/{room_key}
    let rooms_router = Router::<R>::new()
        .route("/", get(get_room_info_route::<R>))
        .route("/connect", get(connect_route::<R>));

    Router::<R>::new().nest("/@/{org_slug}/{app_name}/{room_key}", rooms_router)
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
    app: App,
) -> Result<Json<PublicRoomInfo>, ErrorResponse> {
    let room = host
        .room_storage()
        .get_by_key(&app, app.parse_room_key(&path.room_key)?)
        .await
        .ok_or_else(|| ErrorResponse::not_found(Some("Room with requested key not found.")))?;

    let meta = room
        .meta_storage()
        .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Public)
        .await;

    Ok(Json(PublicRoomInfo { meta }))
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
    app: App,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let room_creation_strategy = app.config().rooms;
    let room_key = app.parse_room_key(&path.room_key)?;

    // Check if the request is allowed to connect to/create a room if it doesn't exist yet.
    pre_create_room_auth_check(&query_params, app.config())?;

    let room = match host.room_storage().get_by_key(&app, room_key).await {
        Some(room) => room, // Room is already created, proceed to connect.
        None => {
            // We need to start the room if it exists, or create it if it doesn't exist and the strategy
            // allows for it.
            if let RoomCreationStrategy::AutoCreate = room_creation_strategy {
                // Create the room automatically since it doesn't exist and the strategy allows for it.
                host.room_storage()
                    .create(CreateRoomOptions {
                        app: &app,
                        creation_strategy: RoomCreationStrategy::AutoCreate,
                        room_key: None,
                        meta: None,
                    })
                    .await?
            } else {
                // The room doesn't exist and the strategy doesn't allow for automatic creation.
                return Err(ErrorResponse::not_found(Some(
                    "Room with requested key not found.",
                )));
            }
        }
    };

    let auth_data = get_auth_data(&query_params, &app, &room)?;
    let response = do_ws_upgrade(WsUpgradeOptions {
        ws,
        room,
        auth_data,
    })
    .await;

    Ok(response)
}
