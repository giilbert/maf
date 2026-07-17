//! Shared API routes for MAF Platform. This includes the WebSocket connection route, room and
//! user management management routes, and any other routes that are used to manage MAF Platform
//! resources.
//!
//! **If the MAF Platform client library can interact with the route, the route should be defined
//! here.** If a route is only used internally by MAF Platform (e.g. uploading user apps), it is not
//! defined here.
//!
//! This module is also documentation for the MAF Platform API. API schemas and behavior should be
//! documented adjacent to the route handlers.

mod gateway;
mod service;

use axum::Router;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use maf_schemas::ErrorResponse;

use crate::server::RoomHostImpl;
use crate::server::app::App;
use crate::server::routes::gateway::create_gateway_router;
use crate::server::routes::service::create_service_v1_router;

/// Creates an axum router with all the MAF Platform API routes defined in this crate. Users that
/// want to implement a MAF Platform host should **merge** this router into their own axum router to
/// expose the MAF Platform API routes.
pub fn create_router<R: RoomHostImpl>(host: &R) -> Router<R> {
    Router::<R>::new()
        .merge(create_gateway_router::<R>(host))
        .nest("/api/v1", create_service_v1_router::<R>(host))
}

/// Middleware that checks if the request is authenticated as a service account for the app
/// specified in the request path. The request path must contain path parameters for `org_slug` and
/// `app_name`, which are used to look up the app and verify the service account's credentials.
///
/// If the path does not have the required path parameters, the middleware will complain with an
/// internal server error. If the service account is not authenticated, the middleware will return
/// a 401 Unauthorized error response.
pub async fn service_account_auth_middleware<R: RoomHostImpl>(
    State(host): State<R>,
    app: App,
    request: Request,
    next: Next,
) -> Result<Response, ErrorResponse> {
    let is_api_client_valid = host.validate_api_key(&app, &request).await?;
    if !is_api_client_valid {
        return Err(ErrorResponse::unauthorized(None));
    }

    Ok(next.run(request).await)
}
