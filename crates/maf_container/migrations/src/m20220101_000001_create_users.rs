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
            .create_foreign_key(
                ForeignKey::create()
                    .from(Token::Table, Token::UserId)
                    .name("fk_user_id")
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::NoAction)
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
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Token::Table).to_owned())
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
    Token,
}
