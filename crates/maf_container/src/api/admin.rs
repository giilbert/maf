use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::{self, Next},
    response::Response,
    Router,
};
use sea_orm::EntityTrait;

use crate::storage::db::user;

use super::{error::ErrorResponse, state::AppState};

pub fn create_admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users", axum::routing::get(get_users))
        .layer(middleware::from_fn_with_state(state, authenticate_request))
}

async fn authenticate_request(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ErrorResponse> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|h| h.trim_start_matches("Bearer ").to_string())
        .ok_or_else(|| ErrorResponse::unauthorized(Some("Missing Authorization header")))?;

    let user = state
        .get_token_user(&token)
        .await?
        .ok_or_else(|| ErrorResponse::unauthorized(Some("Invalid token")))?;

    let permissions = user.permissions;
    if !permissions.is_admin() {
        return Err(ErrorResponse::unauthorized(Some(
            "User does not have admin permissions",
        )));
    }

    req.extensions_mut().insert(AdminUser { user });

    Ok(next.run(req).await)
}

#[derive(Debug, Clone)]
struct AdminUser {
    user: user::Model,
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<AdminUser>()
            .cloned()
            .ok_or_else(|| ErrorResponse::unauthorized(Some("Missing admin user")))?;

        Ok(user)
    }
}

async fn get_users(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ErrorResponse> {
    let users = user::Entity::find().all(&state.db).await?;
    Ok(axum::Json(users))
}
