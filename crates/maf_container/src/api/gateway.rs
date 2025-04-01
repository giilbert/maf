use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::StreamExt;
use serde::Deserialize;

use super::{
    connection::Connection,
    room::Room,
    state::AppState,
    user_app::{RoomCreationStrategy, UserApp},
};

pub fn create_gateway_router(state: AppState) -> Router<AppState> {
    let inner = Router::new().route("/connect", get(connect_route));
    Router::new().nest("/@/{org_slug}/{app_slug}", inner)
}

#[derive(Deserialize)]
pub struct ConnectQueryParams {}

async fn connect_route(
    State(state): State<AppState>,
    Path((org_slug, app_slug)): Path<(String, String)>,
    Query(query_params): Query<ConnectQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, axum::http::StatusCode> {
    // TODO: replace logic with actual user app query from the database
    let app = get_user_app();

    let room = match app.room_creation_strategy {
        RoomCreationStrategy::AutoCreate => {
            let room_id = match state
                .auto_created_rooms_by_org_slug
                .get(&org_slug)
                .map(|room_id| room_id.clone())
            {
                Some(room_id) => room_id,
                None => {
                    let room = Room::new(&state).await.map_err(|e| {
                        tracing::warn!("failed to create room: {e:?}");
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                    let room_id = room.id;
                    state
                        .auto_created_rooms_by_org_slug
                        .insert(org_slug.clone(), room.id);
                    state.rooms.insert(room_id, room);
                    room_id
                }
            };

            state.rooms.get(&room_id).expect("room not found")
        }
    };

    Ok(ws.on_upgrade(|ws| async move {
        let connection = match Connection::init(ws, query_params).await {
            Ok(connection) => connection,
            Err(error) => {
                tracing::warn!("websocket init failed: {error:?}");
                return;
            }
        };

        match connection.run().await {
            Ok(_) => {
                tracing::info!("websocket connection closed");
            }
            Err(error) => {
                tracing::warn!("websocket connection error: {error:?}");
            }
        }
    }))
}

// TODO:
fn get_user_app() -> UserApp {
    UserApp {
        room_creation_strategy: RoomCreationStrategy::AutoCreate,
    }
}
