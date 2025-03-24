use axum::{extract::Path, Router};

use super::state::AppState;

pub fn create_gateway_router(state: AppState) -> Router<AppState> {
    Router::new().route(
        "/{slug}",
        axum::routing::get(|Path(slug): Path<String>| async move { format!("Hello, {}", slug) }),
    )
}
