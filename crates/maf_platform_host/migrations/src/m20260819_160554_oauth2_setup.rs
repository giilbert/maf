use sea_orm::sea_query::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    Email,
    EmailVerified,
    AvatarUrl,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Session {
    Table,
    Id,
    UserId,
    TokenHash,
    CreatedAt,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Id,
    UserId,
    Provider,
    ProviderUserId,
    AccessToken,
    RefreshToken,
    ExpiresAt,
    Scopes,
    CreatedAt,
    UpdatedAt,
}

enum OAuthProvider {
    Type,
    Google,
}

impl Iden for OAuthProvider {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        match self {
            OAuthProvider::Type => write!(s, "oauth_provider").unwrap(),
            OAuthProvider::Google => write!(s, "google").unwrap(),
        }
    }
}

const ALL_OAUTH_PROVIDERS: [OAuthProvider; 1] = [OAuthProvider::Google];

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(User::Table)
                    .add_column(ColumnDef::new(User::Email).string().not_null().unique_key())
                    .add_column(
                        ColumnDef::new(User::EmailVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .add_column(ColumnDef::new(User::AvatarUrl).string())
                    .add_column(
                        ColumnDef::new(User::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .add_column(
                        ColumnDef::new(User::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .modify_column(ColumnDef::new(User::Username).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Session::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Session::UserId).uuid().not_null())
                    .col(ColumnDef::new(Session::TokenHash).string().not_null())
                    .col(
                        ColumnDef::new(Session::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Session::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-session-user_id")
                            .from(Session::Table, Session::UserId)
                            .to(User::Table, User::Id)
                            // If user is deleted, delete all sessions for that user.
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(OAuthProvider::Type)
                    .values(ALL_OAUTH_PROVIDERS)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Account::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Account::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Account::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(Account::Provider)
                            .enumeration(OAuthProvider::Type, ALL_OAUTH_PROVIDERS),
                    )
                    .col(ColumnDef::new(Account::ProviderUserId).string().not_null())
                    .col(ColumnDef::new(Account::AccessToken).string().not_null())
                    .col(ColumnDef::new(Account::RefreshToken).string())
                    .col(ColumnDef::new(Account::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Account::Scopes)
                            .array(ColumnType::String(StringLen::None))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Account::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Account::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-account-user_id")
                            .from(Account::Table, Account::UserId)
                            .to(User::Table, User::Id)
                            // If user is deleted, delete all accounts for that user.
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        // Unique on (user id, provider user id, provider) to ensure that a user can
                        // only associate one account per OAuth provider.
                        Index::create()
                            .name("idx-account-user_id-provider-user_id-provider")
                            .table(Account::Table)
                            .col(Account::UserId)
                            .col(Account::ProviderUserId)
                            .col(Account::Provider)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(User::Table)
                    .drop_column(User::Email)
                    .drop_column(User::EmailVerified)
                    .drop_column(User::AvatarUrl)
                    .drop_column(User::CreatedAt)
                    .drop_column(User::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Account::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(OAuthProvider::Type).to_owned())
            .await?;

        Ok(())
    }
}
