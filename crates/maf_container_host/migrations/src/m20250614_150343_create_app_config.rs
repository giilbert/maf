use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use sea_orm_migration::prelude::*;

use crate::{entity::app, m20250419_015427_create_apps::App};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(App::Table)
                    .add_column(ColumnDef::new(App::Config).string())
                    .add_column(ColumnDef::new(App::ApiClientId).not_null().string())
                    .add_column(ColumnDef::new(App::ApiSecret).not_null().string())
                    .to_owned(),
            )
            .await?;

        // index (client_id, secret) for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_app_api_client_id_secret")
                    .table(App::Table)
                    .col(App::ApiClientId)
                    .col(App::ApiSecret)
                    .unique()
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();

        for app in app::Entity::find().all(db).await?.into_iter() {
            let (client_id, client_secret) = app::generate_api_client_id_and_secret();

            let mut active: app::ActiveModel = app.into();

            active.api_client_id = Set(client_id);
            active.api_secret = Set(client_secret);

            active.update(db).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(App::Table)
                    .drop_column(App::Config)
                    .drop_column(App::ApiClientId)
                    .drop_column(App::ApiSecret)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_app_api_client_id_secret")
                    .table(App::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
