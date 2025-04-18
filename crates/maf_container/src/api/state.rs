use std::sync::Arc;

use anyhow::Context;
use dashmap::DashMap;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use uuid::Uuid;

use crate::{
    runtime::ContainerRuntime,
    storage::{
        bundle::BundleStorage,
        db::{
            self,
            user::{self, Permissions},
            TxnError,
        },
        repos::user_repo,
    },
};

use super::room::Room;

#[derive(Debug, Clone)]
pub struct AppState {
    pub container_runtime: ContainerRuntime,
    pub auto_created_rooms_by_org_slug: Arc<DashMap<String, Uuid>>,
    pub rooms: Arc<DashMap<Uuid, Room>>,
    pub bundle_storage: BundleStorage,
    pub db: sea_orm::DatabaseConnection,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let container_runtime = ContainerRuntime::init()?;

        let database_url = dotenvy::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not found"))?;
        let db = sea_orm::Database::connect(&database_url).await?;

        tracing::info!("Connected to database `{}`", database_url);

        let state = Self {
            container_runtime,
            auto_created_rooms_by_org_slug: Arc::new(DashMap::new()),
            rooms: Arc::new(DashMap::new()),
            bundle_storage: BundleStorage::new(),
            db,
        };

        state.init_database().await?;

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
}
