use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use maf_container::server::ErrorResponse;
use uuid::Uuid;

use crate::storage::{db::user, repos::user_repo};

use super::state::AppState;

pub async fn authenticate_request(
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

    let user = user_repo::get_token_user(&state.db, &token)
        .await?
        .ok_or_else(|| ErrorResponse::unauthorized(Some("Invalid token")))?;

    req.extensions_mut().insert(AuthedUser { inner: user });

    Ok(next.run(req).await)
}

#[derive(Debug, Clone)]
pub struct AuthedUser {
    inner: user::Model,
}

impl FromRequestParts<AppState> for AuthedUser {
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<AuthedUser>()
            .cloned()
            .ok_or_else(|| ErrorResponse::unauthorized(Some("Missing admin user")))?;

        Ok(user)
    }
}

impl AuthedUser {
    pub fn id(&self) -> Uuid {
        self.inner.id
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn permissions(&self) -> &user::Permissions {
        &self.inner.permissions
    }
}
