use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::Context;
use axum::http::header::AUTHORIZATION;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use maf_core::server::{App, RoomHostImpl, RoomsStorage, UpgradeableRoomHostImpl};
use maf_core::{ContainerRuntime, utils};
use maf_schemas::apps::AppNameAndOrgSlug;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, EntityTrait, QueryFilter, TransactionTrait,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::storage::bundle::BundleStorage;
use crate::storage::db::user::{self, Permissions};
use crate::storage::db::{self, TxnError};
use crate::storage::repos::{app_repo, user_repo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

#[derive(Debug, Clone)]
pub struct AppState(Arc<AppStateInner>);

#[derive(Debug, Clone)]
pub struct WeakAppState(Weak<AppStateInner>);

#[derive(Debug)]
struct AppStateInner {
    /// The database connection.
    db: sea_orm::DatabaseConnection,
    /// Manages the bundles (the *templates* used to create rooms) for different apps.
    bundle_storage: BundleStorage,
    /// Caches the app data for apps that have been loaded from the database. This is used in
    /// [`RoomHostImpl::app`] to avoid loading the same app multiple times from the database in a
    /// super short period of time.
    app_cache: Mutex<lru_cache::LruCache<AppNameAndOrgSlug, maf_core::server::App>>,

    /// Manages the live room state for the server. This is used in [`RoomHostImpl`].
    rooms: RoomsStorage<AppState>,
    /// Runs WebAssembly modules for rooms on the server. This is used in [`RoomHostImpl`].
    container_runtime: ContainerRuntime,

    /// Whether the server is running in development or production mode.
    environment: Environment,
    /// When the server was last active. Used to determine when to shut down the server due to
    /// inactivity.
    last_activity: &'static AtomicU64,
    /// A cancellation token that fires when the server should shut down.
    cancel_server: CancellationToken,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let last_activity = Box::leak(Box::new(AtomicU64::new(utils::now_as_secs())));
        let container_runtime = ContainerRuntime::init(last_activity)?;

        let database_url = dotenvy::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not found"))?;
        let mut db_options = ConnectOptions::new(database_url.clone());
        db_options.connect_timeout(Duration::from_secs(5));
        let db = sea_orm::Database::connect(db_options).await?;
        tracing::info!("Connected to database `{}`", database_url);

        let environment = dotenvy::var("ENVIRONMENT")
            .or_else(|_| Ok("development".to_string()))
            .and_then(|s| match s.to_lowercase().as_str() {
                "development" => Ok(Environment::Development),
                "production" => Ok(Environment::Production),
                actual => anyhow::bail!(
                    "expected `development` or `production` in ENVIRONMENT. got `{actual}`."
                ),
            })?;

        let bundle_storage = BundleStorage::new().await?;

        const APP_CACHE_SIZE: usize = 500;
        let app_cache = Mutex::new(lru_cache::LruCache::new(APP_CACHE_SIZE));

        let state = Self(Arc::new_cyclic(|weak| {
            let weak_state = WeakAppState(weak.clone());
            let rooms = RoomsStorage::new(weak_state);

            AppStateInner {
                db,
                bundle_storage,
                app_cache,
                rooms,
                container_runtime,
                environment,
                last_activity,
                cancel_server: CancellationToken::new(),
            }
        }));

        state.init_database().await?;

        if let Some(timeout) = dotenvy::var("AUTO_SHUTDOWN_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            tracing::info!(
                "Auto shutdown inactive server is enabled. Timeout: {} seconds",
                timeout
            );

            tokio::spawn(state.clone().inactive_shutdown_task(timeout));
        }

        Ok(state)
    }

    /// Get a reference to the database connection.
    pub fn db(&self) -> &sea_orm::DatabaseConnection {
        &self.0.db
    }

    /// Get a reference to the [`BundleStorage`].
    pub fn bundle_storage(&self) -> &BundleStorage {
        &self.0.bundle_storage
    }

    /// Gets the [`Environment`] the server is running in.
    pub fn environment(&self) -> Environment {
        self.0.environment
    }

    /// Gets a reference to the cancellation token that can be used to cancel the server.
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.0.cancel_server
    }

    // TODO: should this run migrations?
    async fn init_database(&self) -> anyhow::Result<()> {
        if let (Some(default_admin_username), Some(default_admin_token)) = (
            dotenvy::var("DEFAULT_ADMIN_USERNAME").ok(),
            dotenvy::var("DEFAULT_ADMIN_TOKEN").ok(),
        ) {
            let does_user_exist = db::user::Entity::find()
                .filter(user::Column::Username.eq(default_admin_username.clone()))
                .one(self.db())
                .await
                .context("Failed to check if user exists")?
                .is_some();

            if does_user_exist {
                tracing::info!(
                    "Default admin user with username `{}` already exists, skipping creation",
                    default_admin_username
                );

                return Ok(());
            }

            tracing::info!(
                "Creating default admin user with username `{}` and token `{}`",
                default_admin_username,
                default_admin_token
            );

            self.db()
                .transaction::<_, (), TxnError>(|txn| {
                    Box::pin(async move {
                        let (user, _org) = user_repo::create_user_with_default_org(
                            txn,
                            user::ActiveModel {
                                id: Set(Uuid::new_v4()),
                                username: Set(default_admin_username.clone()),
                                name: Set(default_admin_username),
                                permissions: Set(Permissions::MANAGE_SERVER),
                            },
                        )
                        .await
                        .context("Failed to create default admin user")?;

                        let _token = db::token::ActiveModel {
                            id: Set(Uuid::new_v4()),
                            user_id: Set(user.id),
                            name: Set("default".to_string()),
                            token: Set(default_admin_token),
                        }
                        .insert(txn)
                        .await
                        .context("Failed to create default admin token")?;

                        Ok(())
                    })
                })
                .await?;
        }

        Ok(())
    }

    async fn inactive_shutdown_task(self, timeout: u64) {
        const CHECK_INTERVAL: u64 = 5; // seconds

        // FIXME: investigate race conditions
        // with the current implementation, there is a race condition where the server is shut down
        // while a request is being processed.
        // FIXME: use fly's api to "cordon" the machine, which will prevent new requests from
        // being sent to it, but will allow the current requests to finish.
        // https://fly.io/docs/machines/api/machines-resource/#route-requests-away-from-or-back-to-a-machine

        loop {
            let last_activity = self
                .0
                .last_activity
                .load(std::sync::atomic::Ordering::Relaxed);

            let now = utils::now_as_secs();

            if now - last_activity > timeout {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(CHECK_INTERVAL)).await;
        }

        tracing::info!("Shutting down server due to inactivity");
        self.0.cancel_server.cancel();
    }

    pub fn update_last_activity(&self) {
        self.0
            .last_activity
            .store(utils::now_as_secs(), Ordering::Relaxed);
    }

    /// Gets the last activity timestamp of the server in seconds since the UNIX epoch.
    pub fn last_activity(&self) -> u64 {
        self.0.last_activity.load(Ordering::Relaxed)
    }
}

impl RoomHostImpl for AppState {
    type WeakRef = WeakAppState;

    fn weak(&self) -> Self::WeakRef {
        WeakAppState(Arc::downgrade(&self.0))
    }

    fn container_runtime(&self) -> &ContainerRuntime {
        &self.0.container_runtime
    }

    fn room_storage(&self) -> &RoomsStorage<Self> {
        &self.0.rooms
    }

    fn update_last_activity(&self) -> anyhow::Result<()> {
        self.update_last_activity();
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn app(&self, id: AppNameAndOrgSlug) -> anyhow::Result<Option<App>> {
        // If the app is already in the cache, return it.
        {
            let mut app_cache = self.0.app_cache.lock().await;
            if let Some(app) = app_cache.get_mut(&id) {
                tracing::debug!("cache hit");
                return Ok(Some(app.clone()));
            }
        }

        // TODO: races when multiple requests for the same app come in at the same time. make a way
        // to communicate that this app is being loaded, and have the other requests wait for it

        // Otherwise, load the app from the database.
        let app = app_repo::get_app_by_name_and_org_slug(self.db(), &id.app, &id.org)
            .await?
            .map(|e| {
                App::from_serialized(
                    &id.app,
                    &id.org,
                    maf_schemas::apps::App {
                        id: e.id,
                        name: e.name,
                        config: e.config,
                        api_client_id: e.api_client_id,
                        api_secret: e.api_secret,
                    },
                )
                .context("failed to deserialize app from database")
            })
            .transpose()?;

        // And cache it for future requests.
        if let Some(app) = &app {
            let mut app_cache = self.0.app_cache.lock().await;
            app_cache.insert(id.clone(), app.clone());
        }

        Ok(app)
    }

    async fn validate_api_key(
        &self,
        app: &App,
        headers: &axum::http::HeaderMap,
    ) -> anyhow::Result<bool> {
        let authorization = match headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.trim_start_matches("Basic ").to_string())
        {
            Some(auth) => auth,
            None => return Ok(false),
        };

        let (client_id, secret) =
            match BASE64_STANDARD
                .decode(authorization)
                .ok()
                .and_then(|bytes| {
                    let string = String::from_utf8_lossy(&bytes);
                    let mut parts = string.splitn(2, ':').map(String::from);
                    Some((parts.next()?, parts.next()?))
                }) {
                Some((client_id, secret)) => (client_id, secret),
                None => return Ok(false),
            };

        if self.environment() == Environment::Development {
            tracing::debug!("skipping API key validation in development mode");
            return Ok(true);
        }

        Ok(app.validate_api_credentials(&client_id, &secret))
    }

    async fn load_bundle_for_app(&self, app: &App) -> anyhow::Result<maf_core::server::Bundle> {
        self.0
            .bundle_storage
            .load_app_bundle(app.config().clone(), app.id())
            .await
            .context("failed to load app bundle")?
            .context("app bundle not found")
    }
}

impl UpgradeableRoomHostImpl<AppState> for WeakAppState {
    fn upgrade(&self) -> Option<AppState> {
        self.0.upgrade().map(AppState)
    }
}
