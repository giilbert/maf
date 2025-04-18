use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::org_member;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "org")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub slug: String,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "org_member::Entity")]
    OrgMember,
}

impl Related<org_member::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrgMember.def()
    }

    fn via() -> Option<RelationDef> {
        Some(org_member::Relation::Org.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
