pub mod connection;
mod gateway;
mod room;
mod state;
mod user_app;

use axum::{routing::get, Router};
use state::AppState;

pub fn create_app() -> anyhow::Result<(AppState, Router)> {
    let state = AppState::new()?;

    let router = Router::<AppState>::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(gateway::create_gateway_router(state.clone()));

    Ok((state.clone(), router.with_state(state)))
}
