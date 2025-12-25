use sea_orm::{
    entity::prelude::*,
    sea_query::{ArrayType, ValueType, ValueTypeErr},
    TryGetable,
};
use serde::{Deserialize, Serialize};

use crate::entity::org;

use super::{org_member, token};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub name: String,
    pub permissions: Permissions,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "token::Entity")]
    Token,
    #[sea_orm(has_many = "org_member::Entity")]
    OrgMember,
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

impl Into<Value> for Permissions {
    fn into(self) -> Value {
        Value::BigInt(Some(self.bits()))
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
