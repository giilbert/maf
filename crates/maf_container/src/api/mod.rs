mod admin;
mod auth;
pub mod connection;
mod error;
mod gateway;
mod room;
mod state;
mod user_app;

use auth::authenticate_request;
use axum::{middleware, routing::get, Router};
use state::AppState;

pub async fn create_app() -> anyhow::Result<(AppState, Router)> {
    let state = AppState::new().await?;

    let router = Router::<AppState>::new()
        .route("/", get(|| async { "Hello, World!" }))
        .nest("/api", create_api_router(state.clone()))
        .merge(gateway::create_gateway_router(state.clone()));

    Ok((state.clone(), router.with_state(state)))
}

fn create_api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/admin", admin::create_admin_router(state.clone()))
        .nest("/apps", user_app::create_user_app_router(state.clone()))
        .layer(middleware::from_fn_with_state(state, authenticate_request))
}
