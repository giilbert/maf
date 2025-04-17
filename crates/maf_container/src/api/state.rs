use std::sync::Arc;

use anyhow::Context;
use bitflags::Flags;
use dashmap::DashMap;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter, TransactionTrait,
};
use uuid::Uuid;

use crate::{
    runtime::ContainerRuntime,
    storage::{
        bundle::BundleStorage,
        db::{
            self,
            user::{self, Permissions},
        },
    },
};

use super::room::Room;

#[derive(Debug, Clone)]
pub struct AppState {
    pub container_runtime: ContainerRuntime,
    pub auto_created_rooms_by_org_slug: Arc<DashMap<String, Uuid>>,
    pub rooms: Arc<DashMap<Uuid, Room>>,
    pub bundle_storage: BundleStorage,
    pub db: DatabaseConnection,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let container_runtime = ContainerRuntime::init()?;

        Ok(Self {
            container_runtime,
            auto_created_rooms_by_org_slug: Arc::new(DashMap::new()),
            rooms: Arc::new(DashMap::new()),
            bundle_storage: BundleStorage::new(),
            db: Self::init_database().await?,
        })
    }

    // TODO: should this run migrations?
    async fn init_database() -> anyhow::Result<DatabaseConnection> {
        let database_url = dotenvy::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not found"))?;
        let db: DatabaseConnection = Database::connect(&database_url).await?;

        tracing::info!("Connected to database `{}`", database_url);

        if let (Some(default_admin_username), Some(default_admin_token)) = (
            dotenvy::var("DEFAULT_ADMIN_USERNAME").ok(),
            dotenvy::var("DEFAULT_ADMIN_TOKEN").ok(),
        ) {
            let does_user_exist = db::user::Entity::find()
                .filter(user::Column::Username.eq(default_admin_username.clone()))
                .one(&db)
                .await
                .context("Failed to check if user exists")?
                .is_some();

            if does_user_exist {
                tracing::info!(
                    "Default admin user with username `{}` already exists, skipping creation",
                    default_admin_username
                );
                return Ok(db);
            }

            tracing::info!(
                "Creating default admin user with username `{}` and token `{}`",
                default_admin_username,
                default_admin_token
            );

            let txn = db.begin().await?;

            let user = db::user::ActiveModel {
                id: Set(Uuid::new_v4()),
                username: Set(default_admin_username.clone()),
                name: Set(default_admin_username),
                permissions: Set(Permissions::empty()),
            }
            .insert(&db)
            .await
            .context("Failed to create default admin user")?;

            let _token = db::token::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(user.id),
                name: Set("default".to_string()),
                token: Set(default_admin_token),
            }
            .insert(&db)
            .await
            .context("Failed to create default admin token")?;

            txn.commit().await?;
        }

        Ok(db)
    }
}
