use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
    Router,
};
use sea_orm::EntityTrait;

use crate::storage::db::user::{self};

use super::{auth::AuthedUser, error::ErrorResponse, state::AppState};

pub fn create_admin_router(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users", axum::routing::get(get_users))
        .layer(axum::middleware::from_fn(
            |req: Request, next: Next| async move {
                if !req
                    .extensions()
                    .get::<AuthedUser>()
                    .is_some_and(|user: &AuthedUser| user.permissions().is_admin())
                {
                    ErrorResponse::forbidden(Some(
                        "You are not authorized to access this resource.",
                    ))
                    .into_response()
                } else {
                    next.run(req).await
                }
            },
        ))
}

async fn get_users(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ErrorResponse> {
    let users = user::Entity::find().all(&state.db).await?;
    Ok(axum::Json(users))
}
