use sea_orm::ActiveValue::Set;
use sea_orm_migration::prelude::*;
use serde::Deserialize;

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
                    // These two columns are initially null, but will be populated later
                    .add_column(ColumnDef::new(App::ApiClientId).string())
                    .add_column(ColumnDef::new(App::ApiSecret).string())
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

        {
            use sea_orm::entity::prelude::*;
            use serde::Deserialize;

            // Use a temporary entity to update existing apps with API client ID and secret
            #[derive(Clone, Debug, PartialEq, Eq, Deserialize, DeriveEntityModel)]
            #[sea_orm(table_name = "app")]
            pub struct Model {
                #[sea_orm(primary_key)]
                pub id: Uuid,
                pub api_client_id: Option<String>,
                pub api_secret: Option<String>,
            }

            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}

            impl ActiveModelBehavior for ActiveModel {}

            for app in Entity::find().all(db).await?.into_iter() {
                let (client_id, client_secret) = app::generate_api_client_id_and_secret();

                let mut active: ActiveModel = app.into();

                active.api_client_id = Set(Some(client_id));
                active.api_secret = Set(Some(client_secret));

                active.update(db).await?;
            }
        }

        // Make the api client id and secret non-nullable
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(App::Table)
                    .modify_column(ColumnDef::new(App::ApiClientId).string().not_null())
                    .modify_column(ColumnDef::new(App::ApiSecret).string().not_null())
                    .to_owned(),
            )
            .await?;

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
