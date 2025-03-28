use axum::{
    extract::{Path, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};

use super::state::AppState;

pub fn create_gateway_router(state: AppState) -> Router<AppState> {
    let inner = Router::new().route("/connect", get(app_route));
    Router::new().nest("/@/{org_slug}/{app_slug}", inner)
}

async fn app_route(
    Path((org_slug, app_slug)): Path<(String, String)>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(|ws| async move {
        tracing::info!("websocket upgrade");
    })
}
