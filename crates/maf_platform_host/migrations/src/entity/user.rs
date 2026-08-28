use sea_orm::TryGetable;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ValueType, ValueTypeErr};
use serde::{Deserialize, Serialize};

use super::{account, org_member, session, token};
use crate::entity::org;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: Option<String>,
    pub name: String,
    pub permissions: Permissions,

    #[sea_orm(default_value = "None")]
    pub avatar_url: Option<String>,
    #[sea_orm(unique)]
    pub email: String,
    #[sea_orm(default_value = false)]
    pub email_verified: bool,

    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "token::Entity")]
    Token,
    #[sea_orm(has_many = "org_member::Entity")]
    OrgMember,
    #[sea_orm(has_many = "account::Entity")]
    Account,
    #[sea_orm(has_many = "session::Entity")]
    Session,
}

impl Related<token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Token.def()
    }
}

impl Related<org::Entity> for Entity {
    fn to() -> RelationDef {
        org_member::Relation::Org.def()
    }

    fn via() -> Option<RelationDef> {
        Some(org_member::Relation::User.def().rev())
    }
}

impl Related<org_member::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrgMember.def()
    }
}

impl Related<account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl Related<session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

bitflags::bitflags! {
    /// Permissions for a user on the server.
    ///
    /// - `MANAGE_SERVER` allows the user to have full administrative privileges on the server,
    ///   including managing users, apps, and server settings.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct Permissions: i64 {
        const MANAGE_SERVER = 1 << 0;
    }
}

/// Implement SeaORM traits for `Permissions` to allow it to be used as a database value type.
impl ValueType for Permissions {
    fn type_name() -> String {
        "Permissions".to_string()
    }

    fn array_type() -> ArrayType {
        ArrayType::BigUnsigned
    }

    fn column_type() -> ColumnType {
        ColumnType::BigUnsigned
    }

    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        if let Value::BigInt(Some(v)) = v {
            Ok(Permissions::from_bits_truncate(v))
        } else {
            Err(ValueTypeErr)
        }
    }
}

impl From<Permissions> for Value {
    fn from(val: Permissions) -> Self {
        Value::BigInt(Some(val.bits()))
    }
}

impl TryGetable for Permissions {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        let v = res.try_get_by::<i64, I>(index)?;
        Ok(Permissions::from_bits_truncate(v))
    }
}

impl Permissions {
    pub fn is_admin(&self) -> bool {
        self.contains(Permissions::MANAGE_SERVER)
    }
}

impl From<Model> for maf_schemas::admin::UserAdminView {
    fn from(val: Model) -> Self {
        maf_schemas::admin::UserAdminView {
            id: val.id,
            username: val.username,
            name: val.name,
            email: val.email,
            permissions: format!("{:?}", val.permissions),
        }
    }
}
