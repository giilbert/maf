use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};

use anyhow::Context;
use maf_container::ContainerRuntime;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    storage::{
        bundle::BundleStorage,
        db::{
            self,
            user::{self, Permissions},
            TxnError,
        },
        repos::user_repo,
    },
    utils,
};

use super::room::Room;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub environment: Environment,
    pub container_runtime: ContainerRuntime,
    pub auto_created_rooms_by_org_slug: Arc<RwLock<HashMap<String, Uuid>>>,
    pub rooms: Arc<RwLock<HashMap<Uuid, Room>>>,
    pub bundle_storage: BundleStorage,
    pub db: sea_orm::DatabaseConnection,
    pub last_activity: &'static AtomicU64,
    pub cancel_server: CancellationToken,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let container_runtime = ContainerRuntime::init()?;

        let database_url = dotenvy::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not found"))?;
        let db = sea_orm::Database::connect(&database_url).await?;

        tracing::info!("Connected to database `{}`", database_url);

        let state = Self {
            environment: dotenvy::var("ENVIRONMENT")
                .map_err(anyhow::Error::from)
                .and_then(|s| match s.to_lowercase().as_str() {
                    "development" => Ok(Environment::Development),
                    "production" => Ok(Environment::Production),
                    actual => anyhow::bail!(
                        "expected `development` or `production` in ENVIRONMENT. got `{actual}`."
                    ),
                })?,
            container_runtime,
            auto_created_rooms_by_org_slug: Default::default(),
            rooms: Default::default(),
            bundle_storage: BundleStorage::new().await?,
            db,
            last_activity: Box::leak(Box::new(AtomicU64::new(utils::now_as_secs()))),
            cancel_server: CancellationToken::new(),
        };

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

    // TODO: should this run migrations?
    async fn init_database(&self) -> anyhow::Result<()> {
        if let (Some(default_admin_username), Some(default_admin_token)) = (
            dotenvy::var("DEFAULT_ADMIN_USERNAME").ok(),
            dotenvy::var("DEFAULT_ADMIN_TOKEN").ok(),
        ) {
            let does_user_exist = db::user::Entity::find()
                .filter(user::Column::Username.eq(default_admin_username.clone()))
                .one(&self.db)
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

            self.db
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
                .last_activity
                .load(std::sync::atomic::Ordering::Relaxed);

            let now = utils::now_as_secs();

            if now - last_activity > timeout {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(CHECK_INTERVAL)).await;
        }

        tracing::info!("Shutting down server due to inactivity");
        self.cancel_server.cancel();
    }

    pub fn update_last_activity(&self) {
        self.last_activity
            .store(utils::now_as_secs(), std::sync::atomic::Ordering::Relaxed);
    }
}
