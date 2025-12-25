use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::IntoResponse,
    Router,
};
use maf_schemas::error::ErrorResponse;
use migrations::entity::org;
use sea_orm::EntityTrait;

use crate::{api::auth::authenticate_user_request, storage::db::user};

use super::{auth::AuthedUser, state::AppState};

/// Middleware that asserts the user is an admin.
pub async fn assert_admin(req: Request, next: Next) -> impl IntoResponse {
    if !req
        .extensions()
        .get::<AuthedUser>()
        .is_some_and(|user: &AuthedUser| user.permissions().is_admin())
    {
        ErrorResponse::forbidden(Some("You are not authorized to access this resource."))
            .into_response()
    } else {
        next.run(req).await
    }
}

pub fn create_admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users", axum::routing::get(get_users))
        .layer(middleware::from_fn(assert_admin))
        .layer(middleware::from_fn_with_state(
            state,
            authenticate_user_request,
        ))
}

#[derive(serde::Serialize)]
struct UserWithOrgs {
    #[serde(flatten)]
    user: user::Model,
    orgs: Vec<org::Model>,
}

async fn get_users(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ErrorResponse> {
    let users = user::Entity::find()
        .find_with_related(org::Entity)
        .all(&state.db)
        .await?
        .into_iter()
        .map(|(user_model, org_models)| UserWithOrgs {
            user: user_model,
            orgs: org_models,
        })
        .collect::<Vec<_>>();

    Ok(axum::Json(users))
}
