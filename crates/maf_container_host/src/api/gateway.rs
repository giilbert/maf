use axum::{
    extract::{ws::WebSocket, Path, Query, State, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use serde::Deserialize;

use crate::{api::ErrorResponse, storage::repos::app_repo};

use super::{
    connection::Connection,
    room::Room,
    state::{AppState, Environment},
    user_app::RoomCreationStrategy,
};

pub fn create_gateway_router(_state: AppState) -> Router<AppState> {
    let inner = Router::new().route("/connect", get(connect_route));
    Router::new().nest("/@/{org_slug}/{app_slug}", inner)
}

#[derive(Deserialize)]
pub struct ConnectQueryParams {}

async fn connect_route(
    State(state): State<AppState>,
    Path((org_slug, app_name)): Path<(String, String)>,
    Query(query_params): Query<ConnectQueryParams>,
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
            let room_id = match state
                .auto_created_rooms_by_org_slug
                .read()
                .await
                .get(&org_slug)
                .map(|room_id| room_id.clone())
            {
                Some(room_id) => room_id,
                None => {
                    let (room, mut container) = match app {
                        Ok(app) => Room::new(&state, app.id).await?,
                        Err(_) if state.environment == Environment::Development => {
                            tracing::info!(
                                "App not found. Defaulting to test app (development only)"
                            );
                            Room::new_test(&state).await?
                        }
                        Err(e) => return Err(e),
                    };

                    let room_id = room.id;
                    state
                        .auto_created_rooms_by_org_slug
                        .write()
                        .await
                        .insert(org_slug.clone(), room.id);

                    state.rooms.write().await.insert(room_id, room);

                    let state = state.clone();
                    tokio::spawn(async move {
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

    Ok(ws.on_upgrade(|ws| async move {
        async fn try_init(
            ws: WebSocket,
            query_params: ConnectQueryParams,
            room: Room,
        ) -> anyhow::Result<Connection> {
            let connection = Connection::init(ws, query_params).await?;
            let handle = connection.handle();
            room.add_connection(handle).await?;
            Ok(connection)
        }

        let connection = match try_init(ws, query_params, room).await {
            Ok(connection) => connection,
            Err(error) => {
                tracing::warn!("failed to initialize connection: {error:?}");
                return;
            }
        };

        match connection.run().await {
            Ok(_) => tracing::info!("websocket connection closed"),
            Err(error) => tracing::warn!("websocket connection error: {error:?}"),
        }
    }))
}
