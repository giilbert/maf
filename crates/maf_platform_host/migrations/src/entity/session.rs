use sea_orm::entity::prelude::*;

use super::user;

/// Represents a session for a user on the MAF Platform.
///
/// This is different from [`super::token`] entity, which is used for CLI access.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "session")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    /// Foreign key to the `user.id` field.
    pub user_id: Uuid,
    /// The session token that is used to authenticate the user with the MAF Platform.
    pub token_hash: String,

    /// When the session was created.
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,
    /// When the session expires.
    pub expires_at: DateTimeUtc,
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
