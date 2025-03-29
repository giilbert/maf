use axum::{
    extract::{Path, Query, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use futures_util::StreamExt;
use serde::Deserialize;

use super::{connection::Connection, state::AppState};

pub fn create_gateway_router(state: AppState) -> Router<AppState> {
    let inner = Router::new().route("/connect", get(connect_route));
    Router::new().nest("/@/{org_slug}/{app_slug}", inner)
}

#[derive(Deserialize)]
pub struct ConnectQueryParams {}

async fn connect_route(
    Path((org_slug, app_slug)): Path<(String, String)>,
    Query(query_params): Query<ConnectQueryParams>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(|ws| async move {
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
    })
}
