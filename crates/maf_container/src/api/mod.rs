mod gateway;
mod state;

use axum::{routing::get, Router};
use state::AppState;

pub fn create_app() -> (AppState, Router) {
    let state = AppState::new();

    let router = Router::<AppState>::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(gateway::create_gateway_router(state.clone()));

    (state.clone(), router.with_state(state))
}
