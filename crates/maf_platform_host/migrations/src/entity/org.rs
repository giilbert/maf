use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::{app, org_member};
use crate::entity::user;

pub type OrgSlug = String;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "org")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub slug: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "org_member::Entity")]
    OrgMember,
    #[sea_orm(has_many = "app::Entity")]
    App,
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        org_member::Relation::User.def()
    }

    fn via() -> Option<RelationDef> {
        Some(org_member::Relation::Org.def().rev())
    }
}

impl Related<app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl Related<org_member::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrgMember.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for maf_schemas::admin::OrgAdminView {
    fn from(val: Model) -> Self {
        maf_schemas::admin::OrgAdminView {
            name: val.name,
            slug: val.slug,
            is_default: val.is_default,
        }
    }
}
