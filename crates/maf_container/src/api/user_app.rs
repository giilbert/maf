use axum::{
    body::Body,
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sea_orm::{ActiveValue::Set, ModelTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::{
    db::app,
    repos::{app_repo, org_repo, utils::DbErrorExt},
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
    Router::new()
        .route("/", post(create_user_app).get(get_user_apps))
        .route("/{app_name}", get(get_app).delete(delete_app))
        .route("/{app_name}/deployments", post(upload_app_bundle))
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateUserAppRequest {
    pub name: String,
}

async fn create_user_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Json(user_app): Json<CreateUserAppRequest>,
) -> Result<Json<app::Model>, ErrorResponse> {
    if user_app.name.is_empty() {
        return Err(ErrorResponse::bad_request(Some("Name cannot be empty.")));
    }

    if user_app.name.len() > 100 {
        return Err(ErrorResponse::bad_request(Some(
            "Name cannot be longer than 100 characters.",
        )));
    }

    if !user_app.name.chars().all(|c| {
        (c.is_ascii_alphanumeric() && (c.is_ascii_lowercase() || c.is_numeric())) || c == '-'
    }) {
        return Err(ErrorResponse::bad_request(Some(
            "Name can only contain lowercase alphanumeric characters, and hyphens.",
        )));
    }

    // TODO: configure organizations
    let org = crate::storage::repos::org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let app = app_repo::create_app(
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

    Ok(Json(app))
}

async fn get_user_apps(
    State(state): State<AppState>,
    user: AuthedUser,
) -> Result<Json<Vec<app::Model>>, ErrorResponse> {
    let org = org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let apps = app_repo::get_apps_by_org_id(&state.db, org.id).await?;

    Ok(Json(apps))
}

async fn upload_app_bundle(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
    body: Body,
) -> Result<(), ErrorResponse> {
    let org = org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let app = match app_repo::get_app_by_name_and_org_id(&state.db, &app_name, org.id).await? {
        Some(app) => app,
        None => return Err(ErrorResponse::not_found(Some("App not found."))),
    };

    let body = body.into_data_stream();
    state
        .bundle_storage
        .upload_bundle(app.id, body)
        .await
        .map_err(|e| e.error_response())?;

    Ok(())
}

async fn get_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
) -> Result<Json<app::Model>, ErrorResponse> {
    let org = org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    Ok(Json(
        app_repo::get_app_by_name_and_org_id(&state.db, &app_name, org.id)
            .await?
            .ok_or_else(|| ErrorResponse::not_found(Some("App not found.")))?,
    ))
}

async fn delete_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
) -> Result<Json<app::Model>, ErrorResponse> {
    let org = org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let app = match app_repo::get_app_by_name_and_org_id(&state.db, &app_name, org.id).await? {
        Some(app) => app,
        None => return Err(ErrorResponse::not_found(Some("App not found."))),
    };

    app.clone().delete(&state.db).await?;
    state.bundle_storage.delete_app_bundle(app.id).await?;

    Ok(Json(app))
}
