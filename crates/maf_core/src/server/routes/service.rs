use anyhow::Context;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router, middleware};
use maf_schemas::ErrorResponse;
use maf_schemas::apps::{
    MAX_ROOM_KEY_LENGTH, RoomCreationStrategy, RoomKey, ServiceCreateRoomOptions, ServiceRoomInfo,
};
use uuid::Uuid;

use crate::server::RoomHostImpl;
use crate::server::app::App;
use crate::server::room_storage::CreateRoomOptions;
use crate::server::routes::service_account_auth_middleware;

/// Create the router for the MAF Platform API routes that are used by service accounts to create
/// and manage rooms. For routes that clients use to interface with MAF, see
/// [`super::gateway::create_gateway_router`].
///
/// This router should be merged into the main axum router at `/api/v1` to expose the service API
/// routes.
pub fn create_service_v1_router<R: RoomHostImpl>(host: &R) -> Router<R> {
    let service_account_auth =
        middleware::from_fn_with_state(host.clone(), service_account_auth_middleware::<R>);

    let rooms_router = Router::<R>::new()
        .route(
            "/",
            get(service_v1_get_rooms_route::<R>)
                .post(service_v1_create_room_route::<R>)
                .put(service_v1_fetch_room_route::<R>),
        )
        .route("/{room_key}", get(service_v1_get_room_route::<R>));

    let apps_router = Router::<R>::new()
        .nest("/{org_slug}/{app_name}/rooms", rooms_router)
        .route_layer(service_account_auth);

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
    app: App,
) -> Json<Vec<ServiceRoomInfo>> {
    let mut rooms = vec![];

    for room in host.room_storage().get_rooms_for_app(&app).await {
        rooms.push(room.service_room_info().await);
    }

    Json(rooms)
}

/// GET `/api/v1/apps/{org_slug}/{app_name}/rooms/{room_key}`
///
/// Returns information about a specific room for the specified app. This route is used by service
/// accounts to manage rooms for an app. This is different from the client-facing route that returns
/// public information about a room.
async fn service_v1_get_room_route<R: RoomHostImpl>(
    State(host): State<R>,
    app: App,
    Path((_org_slug, _app_name, room_key)): Path<(String, String, RoomKey)>,
) -> Result<Json<ServiceRoomInfo>, ErrorResponse> {
    let room = host
        .room_storage()
        .get_by_key(&app, room_key)
        .await
        .ok_or_else(|| ErrorResponse::not_found(Some("Room with requested key not found.")))?;

    Ok(Json(room.service_room_info().await))
}

trait ValidateCreateRoomOptions {
    fn validate(&self, app: &App) -> Result<(), ErrorResponse>;
}

impl ValidateCreateRoomOptions for ServiceCreateRoomOptions {
    fn validate(&self, app: &App) -> Result<(), ErrorResponse> {
        let room_creation_strategy = app.config().rooms;

        // TODO: currently autocreated rooms cannot be created through the service API. change? it can
        // be a way to "prepare" a room for a user before they connect to it.
        if room_creation_strategy != RoomCreationStrategy::AuthenticatedApiRequest {
            return Err(ErrorResponse::bad_request(Some(
                "Autocreated rooms cannot be created through the service API.",
            )));
        }

        // Validate the custom room key if one is provided.
        if let Some(key) = &self.key {
            // This key is reserved for autocreated rooms.
            if key == "default" {
                return Err(ErrorResponse::bad_request(Some(
                    "Room key cannot be 'default'.",
                )));
            }

            if key.is_empty() {
                return Err(ErrorResponse::bad_request(Some(
                    "Room key cannot be empty.",
                )));
            }

            if key.len() > MAX_ROOM_KEY_LENGTH {
                return Err(ErrorResponse::bad_request(Some(concat!(
                    "Room key cannot be longer than ",
                    stringify!(MAX_ROOM_KEY_LENGTH),
                    " characters."
                ))));
            }

            if Uuid::parse_str(key).is_ok() {
                return Err(ErrorResponse::bad_request(Some(
                    "Room key cannot be a valid UUID.",
                )));
            }
        }

        Ok(())
    }
}

/// POST `/api/v1/apps/{org_slug}/{app_name}/rooms`
///
/// Creates a new room for the specified app and returns information about the newly created room.
/// If a room with the requested key already exists or is being created, this will error.
///
/// TODO: the "is being created" error is being handled as a internal server error, but it should be
/// a conflict error.
async fn service_v1_create_room_route<R: RoomHostImpl>(
    State(host): State<R>,
    app: App,
    Json(body): Json<ServiceCreateRoomOptions>,
) -> Result<Json<ServiceRoomInfo>, ErrorResponse> {
    body.validate(&app)?;

    let room = host
        .room_storage()
        .check_and_create(CreateRoomOptions {
            app: &app,
            creation_strategy: RoomCreationStrategy::AuthenticatedApiRequest,
            meta: body.meta,
            room_key: body.key,
            should_return_existing: false,
        })
        .await?
        .ok_or_else(|| ErrorResponse::conflict(Some("Room with requested key already exists.")))?;

    Ok(Json(room.service_room_info().await))
}

/// PUT `/api/v1/apps/{org_slug}/{app_name}/rooms`
///
/// Creates a new room for the specified app if the room key is not already in use, or returns
/// information about the existing room if it does exist. If the room already exists, this will
/// reset its auto-shutdown timer.
///
/// If two of the same room key are requested at the same time, only one will be created and the
/// other will wait for the first to finish and return the same room.
async fn service_v1_fetch_room_route<R: RoomHostImpl>(
    State(host): State<R>,
    app: App,
    Json(body): Json<ServiceCreateRoomOptions>,
) -> Result<Json<ServiceRoomInfo>, ErrorResponse> {
    body.validate(&app)?;

    let room = host
        .room_storage()
        .check_and_create(CreateRoomOptions {
            app: &app,
            creation_strategy: RoomCreationStrategy::AuthenticatedApiRequest,
            meta: body.meta,
            room_key: body.key,
            should_return_existing: true,
        })
        .await?
        .context("room should not be None")?;

    // Reset the room's auto-shutdown timer since it was just fetched.
    // XXX: this probably races
    room.mark_activity();

    Ok(Json(room.service_room_info().await))
}
