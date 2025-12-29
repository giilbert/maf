use std::collections::BTreeMap;

use axum::{extract::State, routing::get, Json, Router};
use maf_container::MetaVisibility;
use maf_schemas::{
    apps::{CreateRoomOptions, RoomCreationStrategy, RoomInfo},
    error::ErrorResponse,
};

use crate::dev::{dev_server::DevServerState, rooms::InsertRoom};

pub fn create_platform_api_router() -> Router<DevServerState> {
    Router::new().route(
        "/api/v1/apps/{org}/{app}/rooms",
        get(dev_service_get_rooms).post(dev_server_create_room),
    )
}

async fn dev_service_get_rooms(
    State(state): State<DevServerState>,
) -> Result<Json<Vec<RoomInfo>>, ErrorResponse> {
    let mut rooms = vec![];

    for room in state.rooms.inner.read().await.values() {
        rooms.push(RoomInfo {
            id: room.id.clone(),
            key: room.meta.key.clone(),
            secret: room.meta.secret.clone(),
            meta: room
                .inner
                .container
                .meta
                .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Private)
                .await,
        });
    }

    Ok(Json(rooms))
}

async fn dev_server_create_room(
    State(state): State<DevServerState>,
    Json(options): Json<CreateRoomOptions>,
) -> Result<Json<RoomInfo>, ErrorResponse> {
    let room = state
        .rooms
        .insert(
            &state,
            InsertRoom {
                strategy: RoomCreationStrategy::AuthenticatedApiRequest,
                key: options.key,
            },
        )
        .await?;

    Ok(Json(RoomInfo {
        id: room.id.clone(),
        key: room.meta.key.clone(),
        secret: room.meta.secret.clone(),
        meta: room
            .inner
            .container
            .meta
            .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Private)
            .await,
    }))
}
