use axum::body::Body;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use chrono::Utc;
use maf_schemas::apps::{CreateUserAppRequest, UpdateUserAppRequest};
use maf_schemas::error::ErrorResponse;
use maf_schemas::project_config::ProjectConfigFile;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ModelTrait};
use uuid::Uuid;

use super::auth::AuthedUser;
use super::state::AppState;
use crate::api::auth::authenticate_user_request;
use crate::storage::bundle::BundleError;
use crate::storage::db::app;
use crate::storage::repos::utils::DbErrorExt;
use crate::storage::repos::{app_repo, org_repo};

/// Router for managing user apps.
///
/// These routes are not part of the MAF Platform API, but are used by the CLI/dashboard to manage
/// user apps.
pub fn create_user_app_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_user_app).get(get_user_apps))
        .route(
            "/{app_name}",
            get(get_app).delete(delete_app).post(update_app),
        )
        .route("/{app_name}/deployments", post(upload_app_bundle))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate_user_request,
        ))
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

    if user_app
        .config
        .as_ref()
        .is_some_and(|config| config.len() > 2000)
    {
        return Err(ErrorResponse::bad_request(Some(
            "Config cannot be longer than 2000 characters.",
        )));
    }

    // TODO: configure organizations
    let org = crate::storage::repos::org_repo::get_default_org_of_user(state.db(), user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let (api_client_id, api_secret) = app::generate_api_client_id_and_secret();

    let app = app_repo::create_app(
        state.db(),
        app::ActiveModel {
            id: Set(Uuid::new_v4()),
            name: Set(user_app.name.clone()),
            org_id: Set(org.id),
            updated_at: Set(Utc::now().naive_utc()),
            config: Set(user_app.config),
            api_client_id: Set(api_client_id),
            api_secret: Set(api_secret),
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
    let org = org_repo::get_default_org_of_user(state.db(), user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let apps = app_repo::get_apps_by_org_id(state.db(), org.id).await?;

    Ok(Json(apps))
}

async fn upload_app_bundle(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
    body: Body,
) -> Result<(), ErrorResponse> {
    let org = org_repo::get_default_org_of_user(state.db(), user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let app = match app_repo::get_app_by_name_and_org_id(state.db(), &app_name, org.id).await? {
        Some(app) => app,
        None => return Err(ErrorResponse::not_found(Some("App not found."))),
    };

    let app_config = app
        .config
        .and_then(|config| toml::from_str::<ProjectConfigFile>(&config).ok())
        .unwrap_or_else(|| ProjectConfigFile::default_for(&app.name));

    let body = body.into_data_stream();
    state
        .bundle_storage()
        .upload_bundle(app_config, app.id, body)
        .await
        .map_err(|e| e.error_response())?;

    Ok(())
}

/// **POST** `/api/v1/apps/{org}/{app}`
///
/// Updates the app's configuration. Everything except the config is immutable.
async fn update_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
    Json(updated_app): Json<UpdateUserAppRequest>,
) -> Result<Json<app::Model>, ErrorResponse> {
    let org = org_repo::get_default_org_of_user(state.db(), user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let app = match app_repo::get_app_by_name_and_org_id(state.db(), &app_name, org.id).await? {
        Some(app) => app,
        None => return Err(ErrorResponse::not_found(Some("App not found."))),
    };

    if let Some(config) = &updated_app.config {
        // TODO: Refactor: this validation logic is duplicated
        if config.len() > 2000 {
            return Err(ErrorResponse::bad_request(Some(
                "Config cannot be longer than 2000 characters.",
            )));
        }

        toml::from_str::<ProjectConfigFile>(config)
            .map_err(|_| ErrorResponse::bad_request(Some("Invalid project config")))?
            .validate()
            .map_err(|e| {
                ErrorResponse::bad_request(Some(&format!(
                    "Failed to validate project config: {}",
                    e
                )))
            })?;
    }

    let mut app_model: app::ActiveModel = app.into();
    app_model.updated_at = Set(Utc::now().naive_utc());
    if let Some(config) = updated_app.config {
        app_model.config = Set(Some(config));
    }

    let updated_app = app_model.update(state.db()).await?;

    Ok(Json(updated_app))
}

async fn get_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
) -> Result<Json<app::Model>, ErrorResponse> {
    let org = org_repo::get_default_org_of_user(state.db(), user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    Ok(Json(
        app_repo::get_app_by_name_and_org_id(state.db(), &app_name, org.id)
            .await?
            .ok_or_else(|| ErrorResponse::not_found(Some("App not found.")))?,
    ))
}

async fn delete_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
) -> Result<Json<app::Model>, ErrorResponse> {
    let org = org_repo::get_default_org_of_user(state.db(), user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let app = match app_repo::get_app_by_name_and_org_id(state.db(), &app_name, org.id).await? {
        Some(app) => app,
        None => return Err(ErrorResponse::not_found(Some("App not found."))),
    };

    app.clone().delete(state.db()).await?;
    match state.bundle_storage().delete_app_bundle(app.id).await {
        Ok(_) | Err(BundleError::FileNotFound) => (),
        Err(e) => return Err(ErrorResponse::from(e)),
    };

    Ok(Json(app))
}
