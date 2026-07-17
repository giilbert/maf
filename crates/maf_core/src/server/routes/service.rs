use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use maf_schemas::ErrorResponse;
use maf_schemas::apps::{RoomCreationStrategy, ServiceCreateRoomOptions, ServiceRoomInfo};

use crate::server::RoomHostImpl;
use crate::server::app::App;
use crate::server::room_storage::CreateRoomOptions;

/// Create the router for the MAF Platform API routes that are used by service accounts to create
/// and manage rooms. For routes that clients use to interface with MAF, see
/// [`super::gateway::create_gateway_router`].
///
/// This router should be merged into the main axum router at `/api/v1` to expose the service API
/// routes.
pub fn create_service_v1_router<R: RoomHostImpl>() -> Router<R> {
    let apps_router = Router::<R>::new().route(
        "/{org_slug}/{app_name}/rooms",
        get(service_v1_get_rooms_route::<R>).post(service_v1_create_room_route::<R>),
    );

    Router::<R>::new().nest("/apps", apps_router)
}

/// GET `/api/v1/apps/{org_slug}/{app_name}/rooms`
///
/// Lists all the rooms for the specified app. This route is used by service accounts to manage
/// rooms for an app. This is different from the client-facing route that returns public information
/// about a room.
///
/// TODO: implement gateway route `/@/{org_slug}/{app_name}/rooms`
async fn service_v1_get_rooms_route<R: RoomHostImpl>(
    State(host): State<R>,
    // FIXME: authentication for service accounts
    app: App,
) -> Json<Vec<ServiceRoomInfo>> {
    let mut rooms = vec![];

    for room in host.room_storage().get_rooms_for_app(&app).await {
        rooms.push(room.service_room_info().await);
    }

    Json(rooms)
}

/// POST `/api/v1/apps/{org_slug}/{app_name}/rooms`
///
/// Creates a new room for the specified app and returns information about the newly created room.
async fn service_v1_create_room_route<R: RoomHostImpl>(
    State(host): State<R>,
    app: App,
    Json(body): Json<ServiceCreateRoomOptions>,
) -> Result<Json<ServiceRoomInfo>, ErrorResponse> {
    let room_creation_strategy = app.config().rooms;

    // TODO: currently autocreated rooms cannot be created through the service API. change? it can
    // be a way to "prepare" a room for a user before they connect to it.
    if room_creation_strategy == RoomCreationStrategy::AutoCreate {
        return Err(ErrorResponse::bad_request(Some(
            "autocreated rooms cannot be created through the service API",
        )));
    }

    let room = host
        .room_storage()
        .create(CreateRoomOptions {
            app: &app,
            creation_strategy: RoomCreationStrategy::AuthenticatedApiRequest,
            meta: body.meta,
            room_key: body.key,
        })
        .await?;

    Ok(Json(room.service_room_info().await))
}
