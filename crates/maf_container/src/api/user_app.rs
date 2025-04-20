use axum::{extract::State, routing::post, Json, Router};
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::{
    db::app,
    repos::{app_repo, utils::DbErrorExt},
};

use super::{auth::AuthedUser, error::ErrorResponse, state::AppState};

// TODO:
pub struct UserApp {
    pub room_creation_strategy: RoomCreationStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomCreationStrategy {
    /// Auto-create a room and put everyone in it
    AutoCreate,
}

pub fn create_user_app_router(_state: AppState) -> Router<AppState> {
    Router::new().route("/", post(create_user_app).get(get_user_apps))
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateUserAppRequest {
    pub name: String,
}

async fn create_user_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Json(user_app): Json<CreateUserAppRequest>,
) -> Result<(), ErrorResponse> {
    if user_app.name.is_empty() {
        return Err(ErrorResponse::bad_request(Some("Name cannot be empty.")));
    }

    if user_app.name.len() > 100 {
        return Err(ErrorResponse::bad_request(Some(
            "Name cannot be longer than 100 characters.",
        )));
    }

    if !user_app
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ErrorResponse::bad_request(Some(
            "Name can only contain alphanumeric characters, dashes, and underscores.",
        )));
    }

    // TODO: configure organizations
    let org = crate::storage::repos::org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    app_repo::create_app(
        &state.db,
        app::ActiveModel {
            id: Set(Uuid::new_v4()),
            name: Set(user_app.name.clone()),
            org_id: Set(org.id),
            updated_at: Set(Utc::now().naive_utc()),
        },
    )
    .await
    .map_err(|e| {
        if e.is_unique_violation() {
            ErrorResponse::conflict(Some("App name already exists."))
        } else {
            ErrorResponse::from(e)
        }
    })?;

    Ok(())
}

async fn get_user_apps(
    State(state): State<AppState>,
    user: AuthedUser,
) -> Result<Json<Vec<app::Model>>, ErrorResponse> {
    let org = crate::storage::repos::org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let apps = crate::storage::repos::app_repo::get_apps_by_org_id(&state.db, org.id).await?;

    Ok(Json(apps))
}
