use std::collections::BTreeMap;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use chrono::Utc;
use maf_core::ContainerResourceLimit;
use maf_core::server::{CreateRoomInnerOptions, RoomInner};
use maf_schemas::apps::{
    AppNameAndOrgSlug, CreateRoomOptions, CreateUserAppRequest, MetaVisibility,
    RoomCreationStrategy, RoomInfo, RoomKeyHash, RoomListQueryParams, RoomQueryResponse,
    UpdateUserAppRequest,
};
use maf_schemas::error::ErrorResponse;
use maf_schemas::project_config::ProjectConfigFile;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ModelTrait};
use uuid::Uuid;

use super::auth::AuthedUser;
use super::state::AppState;
use crate::api::auth::{
    AuthedServiceAccount, authenticate_service_request, authenticate_user_request,
};
use crate::api::rooms::InsertRoom;
use crate::storage::bundle::BundleError;
use crate::storage::db::app;
use crate::storage::repos::utils::DbErrorExt;
use crate::storage::repos::{app_repo, org_repo};

pub fn create_user_app_router(state: AppState) -> Router<AppState> {
    // Router for user operations
    let user_router = Router::new()
        .route("/", post(create_user_app).get(get_user_apps))
        .route(
            "/{app_name}",
            get(get_app).delete(delete_app).post(update_app),
        )
        .route("/{app_name}/deployments", post(upload_app_bundle))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate_user_request,
        ));

    // Router for service account operations
    let service_account_router = Router::new()
        .route(
            "/{org_slug}/{app_name}/rooms",
            get(service_get_rooms).post(service_create_room),
        )
        .layer(middleware::from_fn_with_state(
            state,
            authenticate_service_request,
        ));

    Router::new()
        .merge(user_router)
        .merge(service_account_router)
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
    let org = crate::storage::repos::org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let (api_client_id, api_secret) = app::generate_api_client_id_and_secret();

    let app = app_repo::create_app(
        &state.db,
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

/// **POST** `/api/v1/apps/{org}/{app}`
///
/// Updates the app's configuration. Everything except the config is immutable.
async fn update_app(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(app_name): Path<String>,
    Json(updated_app): Json<UpdateUserAppRequest>,
) -> Result<Json<app::Model>, ErrorResponse> {
    let org = org_repo::get_default_org_of_user(&state.db, user.id())
        .await?
        .ok_or_else(|| ErrorResponse::not_found(Some("No default org found.")))?;

    let app = match app_repo::get_app_by_name_and_org_id(&state.db, &app_name, org.id).await? {
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

    let updated_app = app_model.update(&state.db).await?;

    Ok(Json(updated_app))
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
    match state.bundle_storage.delete_app_bundle(app.id).await {
        Ok(_) | Err(BundleError::FileNotFound) => (),
        Err(e) => return Err(ErrorResponse::from(e)),
    };

    Ok(Json(app))
}

/// **GET** `/api/v1/apps/{org}/{app}/rooms`
///
/// Query parameters can be used to filter rooms:
/// - `by_key`: If provided, only return the room with the specified key.
/// - `by_id`: If provided, only return the room with the specified ID.
///
/// If `by_key` or `by_id` is provided, the response will be a single room if found. If no room can
/// be found when using these filters, a 404 error is returned.
///
/// If no filters are provided, all rooms belonging to the specified app are returned, or an empty
/// list if no rooms exist.
async fn service_get_rooms(
    State(state): State<AppState>,
    service_account: AuthedServiceAccount,
    query: Query<RoomListQueryParams>,
) -> Result<Json<RoomQueryResponse>, ErrorResponse> {
    let app = service_account.app();
    let org = service_account.org();

    if query.by_id.is_some() && query.by_key.is_some() {
        return Err(ErrorResponse::bad_request(Some(
            "Cannot filter by both ID and key simultaneously.",
        )));
    }

    if let Some(query_id) = query.by_id {
        match state.rooms.get(&query_id).await {
            Some(room) if room.meta().app_info == (&app.name, &org.slug) => {
                return Ok(Json(RoomQueryResponse::Single(RoomInfo {
                    id: room.id(),
                    key: room.meta().key.clone(),
                    secret: room.inner().secret().to_string(),
                    meta: room
                        .inner()
                        .meta_storage()
                        .list_values::<BTreeMap<_, _>>(MetaVisibility::Private)
                        .await,
                })));
            }
            _ => {
                return Err(ErrorResponse::not_found(Some(
                    "No room found with the specified ID.",
                )));
            }
        }
    }

    if let Some(query_key) = &query.by_key {
        let room_key_hash = RoomKeyHash {
            app: AppNameAndOrgSlug {
                app: app.name.clone(),
                org: org.slug.clone(),
            },
            key: query_key.clone(),
        };

        match state.rooms.keys.read().await.get(&room_key_hash) {
            Some(room_id) => match state.rooms.get(room_id).await {
                Some(room) => {
                    return Ok(Json(RoomQueryResponse::Single(RoomInfo {
                        id: room.id(),
                        key: room.meta().key.clone(),
                        secret: room.inner().secret().to_string(),
                        meta: room
                            .inner()
                            .meta_storage()
                            .list_values::<BTreeMap<_, _>>(MetaVisibility::Private)
                            .await,
                    })));
                }
                None => {
                    return Err(ErrorResponse::not_found(Some(
                        "No room found with the specified key.",
                    )));
                }
            },
            None => {
                return Err(ErrorResponse::not_found(Some(
                    "No room found with the specified key.",
                )));
            }
        }
    }

    match state
        .rooms
        .api_created_rooms
        .read()
        .await
        .get(&AppNameAndOrgSlug {
            app: app.name.clone(),
            org: org.slug.clone(),
        }) {
        Some(rooms_set) => {
            let mut rooms: Vec<RoomInfo> = vec![];

            for room_id in rooms_set.iter() {
                if let Some(room) = state.rooms.get(room_id).await {
                    rooms.push(RoomInfo {
                        id: room.id(),
                        key: room.meta().key.clone(),
                        secret: room.inner().secret().to_string(),
                        meta: room
                            .inner()
                            .meta_storage()
                            .list_values::<BTreeMap<_, _>>(MetaVisibility::Private)
                            .await,
                    });
                }
            }

            Ok(Json(RoomQueryResponse::Multiple(rooms)))
        }
        None => Ok(Json(RoomQueryResponse::Multiple(vec![]))),
    }
}

/// **POST** `/api/v1/apps/{org}/{app}/rooms`
async fn service_create_room(
    State(state): State<AppState>,
    service_account: AuthedServiceAccount,
    Json(options): Json<CreateRoomOptions>,
) -> Result<Json<RoomInfo>, ErrorResponse> {
    let app = service_account.app();
    let org = service_account.org();

    // Validate options
    if let Some(key) = &options.key {
        if key == "default" || Uuid::parse_str(key).is_ok() {
            return Err(ErrorResponse::bad_request(Some(
                "Key cannot be 'default' or a valid UUID.",
            )));
        }

        if key.len() > 128 {
            return Err(ErrorResponse::bad_request(Some(
                "Key cannot be longer than 128 characters.",
            )));
        }

        // Check for key uniqueness
        if state.rooms.keys.read().await.contains_key(&RoomKeyHash {
            app: AppNameAndOrgSlug {
                app: app.name.clone(),
                org: org.slug.clone(),
            },
            key: key.clone(),
        }) {
            return Err(ErrorResponse::conflict(Some("Room key already exists.")));
        }
    }

    let room_creation_strategy = match &app.config {
        Some(config) => {
            let config: ProjectConfigFile = toml::from_str(config)
                .map_err(|_| ErrorResponse::bad_request(Some("Invalid project config")))?;
            config.rooms
        }
        None => RoomCreationStrategy::AuthenticatedApiRequest,
    };

    if room_creation_strategy != RoomCreationStrategy::AuthenticatedApiRequest {
        return Err(ErrorResponse::forbidden(Some(
            "Authenticated API request is not supported for this app.",
        )));
    }

    let (room, mut container) = RoomInner::new(
        &state.container_runtime,
        CreateRoomInnerOptions {
            bundle: state
                .bundle_storage
                .load_app_bundle(app.id)
                .await
                .map_err(|e| match e {
                    BundleError::InvalidZip => {
                        ErrorResponse::bad_request(Some("Invalid app bundle"))
                    }
                    _ => ErrorResponse::internal_server_error(Some("Failed to load app bundle")),
                })?
                .ok_or_else(|| ErrorResponse::not_found(Some("App bundle not found")))?,
            resource_limit: ContainerResourceLimit::small_defaults(),
            meta: options.meta,
        },
    )
    .await?;

    let room_id = room.id();
    let room_meta = state
        .rooms
        .insert(InsertRoom {
            room: room.clone(),
            strategy: RoomCreationStrategy::AuthenticatedApiRequest,
            app: AppNameAndOrgSlug {
                app: app.name.clone(),
                org: org.slug.clone(),
            },
            key: options.key.unwrap_or_else(|| room_id.to_string()),
        })
        .await;

    let room_id = room_meta.id;
    let state = state.clone();

    container.pass_output();
    container.start_inactive_shutdown_task();

    tokio::spawn(async move {
        if let Err(e) = container.run().await {
            tracing::error!("container {} error: {e:?}", container.room_id());
        }
        tracing::info!("container {} stopped", container.room_id());

        state.rooms.remove(&room_id).await;
    });

    Ok(Json(RoomInfo {
        id: room_id,
        key: room_meta.key.clone(),
        secret: room.secret().to_string(),
        meta: room
            .meta_storage()
            .list_values::<BTreeMap<_, _>>(MetaVisibility::Private)
            .await,
    }))
}
