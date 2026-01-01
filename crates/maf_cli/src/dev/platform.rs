use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use maf_schemas::{
    apps::{
        CreateRoomOptions, MetaVisibility, RoomCreationStrategy, RoomInfo, RoomListQueryParams,
        RoomQueryResponse,
    },
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
    query: Query<RoomListQueryParams>,
) -> Result<Json<RoomQueryResponse>, ErrorResponse> {
    let mut rooms = vec![];

    if let Some(query_id) = &query.by_id {
        if let Some(room) = state.rooms.inner.read().await.get(query_id) {
            let room_info = RoomInfo {
                id: room.id.clone(),
                key: room.meta.key.clone(),
                secret: room.inner.secret.clone(),
                meta: room
                    .inner
                    .container
                    .meta
                    .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Private)
                    .await,
            };
            return Ok(Json(RoomQueryResponse::Single(room_info)));
        } else {
            return Err(ErrorResponse::not_found(Some(
                "No room found with the specified ID.",
            )));
        }
    }

    if let Some(query_key) = &query.by_key {
        for room in state.rooms.inner.read().await.values() {
            if &room.meta.key == query_key {
                return Ok(Json(RoomQueryResponse::Single(RoomInfo {
                    id: room.id.clone(),
                    key: room.meta.key.clone(),
                    secret: room.inner.secret.clone(),
                    meta: room
                        .inner
                        .container
                        .meta
                        .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Private)
                        .await,
                })));
            }
        }

        return Err(ErrorResponse::not_found(Some(
            "No room found with the specified key.",
        )));
    }

    for room in state.rooms.inner.read().await.values() {
        rooms.push(RoomInfo {
            id: room.id.clone(),
            key: room.meta.key.clone(),
            secret: room.inner.secret.clone(),
            meta: room
                .inner
                .container
                .meta
                .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Private)
                .await,
        });
    }

    Ok(Json(RoomQueryResponse::Multiple(rooms)))
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
                meta: options.meta,
            },
        )
        .await?;

    Ok(Json(RoomInfo {
        id: room.id.clone(),
        key: room.meta.key.clone(),
        secret: room.inner.secret.clone(),
        meta: room
            .inner
            .container
            .meta
            .list_values::<BTreeMap<String, serde_json::Value>>(MetaVisibility::Private)
            .await,
    }))
}
