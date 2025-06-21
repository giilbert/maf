//! This module provides middleware for authenticating user requests and extracting relevant user
//! information from the request.
//!
//! Requests are made from two types of clients:
//! - **User clients**: These requests are authenticated using a *user token*. Right now, this is
//!   only the CLI token used by users to manage their resources on MAF.
//! - **Service clients**: These requests are authenticated using *service account credentials*.
//!   This is typically used for user-created backend services that need to interact with the MAF
//!   API (e.g. creating rooms, authenticating users, etc.).

use std::collections::HashMap;

use axum::{
    extract::{FromRequestParts, Path, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use maf_container::server::ErrorResponse;
use migrations::entity::{app, org};
use uuid::Uuid;

use crate::{
    api::state::Environment,
    storage::{
        db::user,
        repos::{app_repo, user_repo},
    },
};

use super::state::AppState;

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

    let user = user_repo::get_token_user(&state.db, &token)
        .await?
        .ok_or_else(|| ErrorResponse::unauthorized(Some("Invalid user token.")))?;

    req.extensions_mut().insert(AuthedUser { inner: user });

    Ok(next.run(req).await)
}

/// Middleware to authenticate service requests. This is used for backend services that need to
/// interact with the MAF API, such as creating rooms or authenticating users.
pub async fn authenticate_service_request(
    State(state): State<AppState>,
    Path(path): Path<HashMap<String, String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ErrorResponse> {
    use base64::prelude::*;

    if state.environment == Environment::Development {
        // In development, we allow requests without authentication for easier testing.

        static SHOWED_SKIP_WARNING: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);

        if !SHOWED_SKIP_WARNING.load(std::sync::atomic::Ordering::Relaxed) {
            SHOWED_SKIP_WARNING.store(true, std::sync::atomic::Ordering::Relaxed);
            tracing::warn!("Skipping service request authentication in development environment.");
        }

        let (app, org) = app_repo::get_app_by_name_and_org_slug_with_org(
            &state.db,
            path.get("app_name").ok_or_else(|| {
                tracing::error!("Middleware error: missing app_name in request path");
                ErrorResponse::internal_server_error(None)
            })?,
            path.get("org_slug").ok_or_else(|| {
                tracing::error!("Middleware error: missing org_slug in request path");
                ErrorResponse::internal_server_error(None)
            })?,
        )
        .await?
        .ok_or_else(|| ErrorResponse::unauthorized(Some("App not found.")))?;

        req.extensions_mut()
            .insert(AuthedServiceAccount { app, org });

        return Ok(next.run(req).await);
    }

    let base64_credentials = get_authorization(&req, "Basic").ok_or_else(|| {
        ErrorResponse::unauthorized(Some(
            "Missing Authorization header with service account credentials.",
        ))
    })?;

    let invalid_credentials_error =
        || ErrorResponse::unauthorized(Some("Invalid service account credentials."));

    let (client_id, secret) = BASE64_STANDARD
        .decode(base64_credentials)
        .ok()
        .and_then(|bytes| {
            let string = String::from_utf8_lossy(&bytes);
            let mut parts = string.splitn(2, ':').map(String::from);
            Some((parts.next()?, parts.next()?))
        })
        .ok_or_else(invalid_credentials_error)?;

    let (app, org) = app_repo::get_app_by_service_account(
        &state.db,
        path.get("org_slug").ok_or_else(|| {
            tracing::error!("Middleware error: missing org_slug in request path");
            ErrorResponse::internal_server_error(None)
        })?,
        &client_id,
        &secret,
    )
    .await?
    .ok_or_else(invalid_credentials_error)?;

    req.extensions_mut()
        .insert(AuthedServiceAccount { app, org });

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

#[derive(Debug, Clone)]
pub struct AuthedServiceAccount {
    app: app::Model,
    org: org::Model,
}

impl FromRequestParts<AppState> for AuthedServiceAccount {
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        let app = parts
            .extensions
            .get::<AuthedServiceAccount>()
            .cloned()
            .ok_or_else(|| ErrorResponse::unauthorized(Some("Missing service account")))?;

        Ok(app)
    }
}

impl AuthedServiceAccount {
    pub fn app(&self) -> &app::Model {
        &self.app
    }

    pub fn org(&self) -> &org::Model {
        &self.org
    }
}
