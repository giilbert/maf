//! This module provides middleware for authenticating user requests and extracting relevant user
//! information from the request.
//!
//! Requests are made from two types of clients:
//! - **User clients**: These requests are authenticated using a *user token*. Right now, this is
//!   only the CLI token used by users to manage their resources on MAF.
//! - **Service clients**: These requests are authenticated using *service account credentials*.
//!   This is typically used for user-created backend services that need to interact with the MAF
//!   API (e.g. creating rooms, authenticating users, etc.). The middleware that handles this is in
//!   [`maf_core::server`].

mod oauth;

use axum::Router;
use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use maf_schemas::error::ErrorResponse;
pub use oauth::OAuthClients;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use super::state::AppState;
use crate::storage::db::user;
use crate::storage::repos::user_repo;

/// Extracts the authorization token from the request headers, removing the specified base prefix.
fn get_authorization(req: &Request, base: &str) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|h| {
            h.trim_start_matches(base)
                .trim_start_matches(" ")
                .to_string()
        })
}

/// Middleware to authenticate user requests. As of right now, this middleware should only be for
/// CLI requests.
pub async fn authenticate_user_request(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ErrorResponse> {
    let token = get_authorization(&req, "Bearer").ok_or_else(|| {
        ErrorResponse::unauthorized(Some("Missing Authorization header with user token."))
    })?;

    let user = user_repo::get_token_user(state.db(), &token)
        .await?
        .ok_or_else(|| ErrorResponse::unauthorized(Some("Invalid user token.")))?;

    req.extensions_mut().insert(AuthedUser { inner: user });

    Ok(next.run(req).await)
}

/// Represents an authenticated user in the system. This is extracted from the request and contains
/// the user's information, such as their ID, name, and permissions.
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

#[allow(dead_code)]
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

pub fn create_auth_router(_state: AppState) -> Router<AppState> {
    let cors = CorsLayer::new()
        .allow_origin(
            "http://localhost:5173"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
            // TODO: add other origins here, like the production panel URL
        )
        .allow_methods(tower_http::cors::Any)
        .allow_headers(vec![
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    Router::new()
        .route("/login", get(oauth::oauth_login))
        .route("/callback/google", get(oauth::oauth_callback_google))
        .layer(cors)
}
