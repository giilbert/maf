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

use axum::Router;

use crate::server::RoomHostImpl;
use crate::server::routes::gateway::create_gateway_router;

/// Creates an axum router with all the MAF Platform API routes defined in this crate. Users that
/// want to implement a MAF Platform host should **merge** this router into their own axum router to
/// expose the MAF Platform API routes.
pub fn create_router<R: RoomHostImpl>() -> Router<R> {
    Router::<R>::new()
        .merge(create_gateway_router::<R>())
        .nest("/api/v1", create_api_v1_router::<R>())
}

/// Create the REST API router for the MAF Platform API routes that are used by services.
fn create_api_v1_router<R: RoomHostImpl>() -> Router<R> {
    Router::<R>::new()
}
