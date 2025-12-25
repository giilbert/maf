use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::IntoResponse,
    Json, Router,
};
use maf_schemas::{admin::UserWithOrgsAdminView, error::ErrorResponse};
use migrations::entity::org;
use sea_orm::{ActiveModelTrait, ActiveValue::*, EntityTrait, TransactionTrait};
use uuid::Uuid;

use crate::{
    api::auth::authenticate_user_request,
    storage::{
        db::{user, TxnError},
        repos::utils::DbErrorExt,
    },
};

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
        .route("/users", axum::routing::get(get_users).post(create_user))
        .layer(middleware::from_fn(assert_admin))
        .layer(middleware::from_fn_with_state(
            state,
            authenticate_user_request,
        ))
}

/// `GET /api/v1/admin/users`
/// Returns a list of users with their associated orgs as [`UserWithOrgsAdminView`].
async fn get_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserWithOrgsAdminView>>, ErrorResponse> {
    let users = user::Entity::find()
        .find_with_related(org::Entity)
        .all(&state.db)
        .await?
        .into_iter()
        .map(|(user_model, org_models)| UserWithOrgsAdminView {
            user: user_model.into(),
            orgs: org_models.into_iter().map(|o| o.into()).collect(),
        })
        .collect::<Vec<_>>();

    Ok(axum::Json(users))
}

/// `POST /api/v1/admin/users` [`maf_schemas::admin::CreateUser`]
/// Creates a new user and their default org, returning the created [`UserWithOrgsAdminView`].
#[tracing::instrument(level = "trace", skip_all)]
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<maf_schemas::admin::CreateUser>,
) -> Result<Json<UserWithOrgsAdminView>, ErrorResponse> {
    tracing::info!("Creating user with username: {:?}", payload);

    if payload.username.trim().is_empty() {
        return Err(ErrorResponse::bad_request(Some(
            "Username cannot be empty.",
        )));
    }

    if payload.name.trim().is_empty() {
        return Err(ErrorResponse::bad_request(Some("Name cannot be empty.")));
    }

    if !payload.username.chars().all(|c| {
        (c.is_ascii_alphanumeric() && (c.is_ascii_lowercase() || c.is_numeric())) || c == '-'
    }) {
        return Err(ErrorResponse::bad_request(Some(
            "Username can only contain lowercase alphanumeric characters and hyphens.",
        )));
    }

    state
        .db
        .transaction::<_, _, TxnError>(|tx| {
            Box::pin(async move {
                let new_user = user::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    username: Set(payload.username.clone()),
                    name: Set(payload.name),
                    permissions: Set(migrations::entity::user::Permissions::empty()),
                };
                let new_org = org::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    name: Set(payload.username.clone()),
                    slug: Set(payload.username),
                    is_default: Set(true),
                };

                let inserted_user = new_user.insert(tx).await?;
                let inserted_org = new_org.insert(tx).await?;

                Ok(UserWithOrgsAdminView {
                    user: inserted_user.into(),
                    orgs: vec![inserted_org.into()],
                })
            })
        })
        .await
        .map(Json)
        .map_err(|e| {
            if e.is_unique_violation() {
                ErrorResponse::conflict(Some("Username already exists."))
            } else {
                ErrorResponse::internal_server_error(Some(&format!("Failed to create user: {}", e)))
            }
        })
}
