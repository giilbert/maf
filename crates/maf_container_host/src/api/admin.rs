use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::IntoResponse,
    Json, Router,
};
use maf_schemas::{
    admin::{DeleteUserAdminView, UserWithOrgsAdminView},
    error::ErrorResponse,
};
use migrations::entity::{org, org_member};
use sea_orm::{
    ActiveModelTrait, ActiveValue::*, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    TransactionTrait,
};
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
        .route("/users/{id}", axum::routing::delete(delete_user))
        .layer(middleware::from_fn(assert_admin))
        .layer(middleware::from_fn_with_state(
            state,
            authenticate_user_request,
        ))
}

/// **GET** `/api/v1/admin/users`
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

/// **POST** `/api/v1/admin/users` [`maf_schemas::admin::CreateUser`]
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

                // Link the created user and org as owner
                org_member::ActiveModel {
                    user_id: Set(inserted_user.id),
                    org_id: Set(inserted_org.id),
                    role: Set("OWNER".to_string()),
                }
                .insert(tx)
                .await?;

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

/// **DELETE** `/api/v1/admin/users/:id`
async fn delete_user(
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<Uuid>,
) -> Result<Json<DeleteUserAdminView>, ErrorResponse> {
    let user_model = user::Entity::find_by_id(user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("User not found.")))?;

    // Prevent deletion of admin users without removing admin permissions first
    if user_model.permissions.is_admin() {
        return Err(ErrorResponse::forbidden(Some(
            "Cannot delete an admin user. Remove admin permissions first.",
        )));
    }

    state
        .db
        .transaction::<_, _, TxnError>(|tx| {
            Box::pin(async move {
                let mut deleted_user = user::Entity::delete_by_id(user_id)
                    .exec_with_returning(tx)
                    .await?;

                let deleted_user = deleted_user
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("User disappeared during deletion"))?;

                let deleted_org_members = org_member::Entity::delete_many()
                    .filter(org_member::Column::UserId.eq(user_id))
                    .exec_with_returning(tx)
                    .await?;

                // If the org has no more members, delete the org as well
                let mut deleted_orgs = Vec::new();
                for org_member in deleted_org_members {
                    let org_id = org_member.org_id;
                    let org_in_use = org_member::Entity::find()
                        .filter(org_member::Column::OrgId.eq(org_id))
                        .count(tx)
                        .await?
                        > 0;

                    if !org_in_use {
                        if let Some(deleted_org) = org::Entity::delete_by_id(org_id)
                            .exec_with_returning(tx)
                            .await?
                            .pop()
                        {
                            deleted_orgs.push(deleted_org.into());
                        }
                    }
                }

                Ok(DeleteUserAdminView {
                    deleted_user: deleted_user.into(),
                    deleted_orgs,
                })
            })
        })
        .await
        .map(Json)
        .map_err(Into::into)
}
