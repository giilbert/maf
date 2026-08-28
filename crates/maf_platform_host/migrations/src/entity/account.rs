use sea_orm::entity::prelude::*;

use super::user;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "account")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    /// Foreign key to the `user.id` field. Note that this is not the user ID that is assigned by
    /// the OAuth provider.
    pub user_id: Uuid,
    /// The OAuth provider that the user used to sign in.
    pub provider: OAuthProvider,
    /// The user ID assigned by the OAuth provider. This is used to identify the user across
    /// different OAuth providers.
    ///
    /// When using Google OAuth, this is the `sub` field in the ID token.
    pub provider_user_id: String,

    /// The access token that is used to authenticate the user with the OAuth provider.
    ///
    /// We don't need to store the access token if we only need to authenticate the user with our
    /// own services (e.g. we're not using the access token to access Google APIs on behalf of the
    /// user). However, it might be useful in the future so we'll keep it.
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// When the access token expires.
    pub expires_at: Option<DateTimeUtc>,

    /// The scopes that the user has granted to the application.
    pub scopes: Vec<String>,

    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "user::Entity")]
    User,
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "oauth_provider")]
pub enum OAuthProvider {
    #[sea_orm(string_value = "google")]
    Google,
}
