use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(pk_uuid(User::Id))
                    .col(string(User::Username).unique_key())
                    .col(string(User::Name))
                    .col(big_unsigned(User::Permissions).default(0))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Token::Table)
                    .if_not_exists()
                    .col(pk_uuid(Token::Id))
                    .col(uuid(Token::UserId))
                    .col(string(Token::Name))
                    .col(string(Token::Token))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_id_token")
                    .table(Token::Table)
                    .col(Token::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_token")
                    .table(Token::Table)
                    .col(Token::Token)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Token::Table, Token::UserId)
                    .name("fk_user_id")
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OrgMember::Table)
                    .if_not_exists()
                    .col(uuid(OrgMember::OrgId))
                    .col(uuid(OrgMember::UserId))
                    .primary_key(
                        Index::create()
                            .name("pk_org_member")
                            .col(OrgMember::OrgId)
                            .col(OrgMember::UserId),
                    )
                    .col(string(OrgMember::Role))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Org::Table)
                    .if_not_exists()
                    .col(pk_uuid(Org::Id))
                    .col(string(Org::Name))
                    .col(string(Org::Slug).unique_key())
                    .col(boolean(Org::IsDefault))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_org_id")
                    .table(OrgMember::Table)
                    .col(OrgMember::OrgId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_id_org_member")
                    .table(OrgMember::Table)
                    .col(OrgMember::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(OrgMember::Table, OrgMember::OrgId)
                    .name("fk_org_id")
                    .to(Org::Table, Org::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(OrgMember::Table, OrgMember::UserId)
                    .name("fk_user_id")
                    .to(User::Table, User::Id)
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
                    .name("fk_user_id")
                    .table(Token::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_org_id")
                    .table(OrgMember::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_user_id")
                    .table(OrgMember::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_token")
                    .table(Token::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_org_id")
                    .table(OrgMember::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Token::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(OrgMember::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Org::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    Name,
    Permissions,
}

#[derive(DeriveIden)]
enum Token {
    Table,
    Id,
    UserId,
    Name,
    #[allow(clippy::enum_variant_names)] // This needs to be a database column
    Token,
}

#[derive(DeriveIden)]
enum OrgMember {
    Table,
    OrgId,
    UserId,
    Role,
}

#[derive(DeriveIden)]
pub enum Org {
    Table,
    Id,
    Name,
    Slug,
    IsDefault,
}
