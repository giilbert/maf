use sea_orm_migration::{prelude::*, schema::*};

use crate::m20220101_000001_setup_users_orgs_apps::Org;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(App::Table)
                    .if_not_exists()
                    .col(pk_uuid(App::Id))
                    .col(string(App::Name))
                    .col(uuid(App::OrgId))
                    .col(timestamp(App::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_app_name_org_id")
                    .table(App::Table)
                    .col(App::Name)
                    .col(App::OrgId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_app_org_id")
                    .from(App::Table, App::OrgId)
                    .to(Org::Table, Org::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(App::Table)
                    .name("fk_app_org_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .table(App::Table)
                    .name("idx_app_name_org_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(App::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum App {
    Table,
    Id,
    Name,
    UpdatedAt,
    OrgId,
}
