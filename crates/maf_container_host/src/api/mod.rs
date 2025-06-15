mod admin;
mod auth;
mod gateway;
mod state;
mod user_app;

use auth::authenticate_request;
use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use state::AppState;

pub async fn create_app() -> anyhow::Result<(AppState, Router)> {
    let state = AppState::new().await?;

    let router = Router::<AppState>::new()
        .route("/", get(|| async { "Hello, World!" }))
        .nest("/api/v1", create_api_v1_router(state.clone()))
        .merge(gateway::create_gateway_router(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            update_last_activity,
        ));

    Ok((state.clone(), router.with_state(state)))
}

fn create_api_v1_router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/admin", admin::create_admin_router(state.clone()))
        .nest("/apps", user_app::create_user_app_router(state.clone()))
        .layer(middleware::from_fn_with_state(state, authenticate_request))
}

async fn update_last_activity(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let response = next.run(request).await;

    if response.status().is_success()
        || response.status().is_informational()
        || response.status().is_redirection()
    {
        state.update_last_activity();
    }

    response
}
