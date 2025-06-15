use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::IntoResponse,
    Router,
};
use maf_container::server::ErrorResponse;
use sea_orm::EntityTrait;

use crate::{api::auth::authenticate_user_request, storage::db::user};

use super::{auth::AuthedUser, state::AppState};

pub fn create_admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users", axum::routing::get(get_users))
        .layer(middleware::from_fn(|req: Request, next: Next| async move {
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
        }))
        .layer(middleware::from_fn_with_state(
            state,
            authenticate_user_request,
        ))
}

async fn get_users(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ErrorResponse> {
    let users = user::Entity::find().all(&state.db).await?;
    Ok(axum::Json(users))
}
