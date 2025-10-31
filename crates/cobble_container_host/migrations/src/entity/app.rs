use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::org;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "app")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub org_id: Uuid,
    pub updated_at: chrono::NaiveDateTime,
    pub config: Option<String>,
    pub api_client_id: String,
    pub api_secret: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "org::Entity",
        from = "Column::OrgId",
        to = "org::Column::Id"
    )]
    Org,
}

impl Related<org::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Org.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub fn generate_api_client_id_and_secret() -> (String, String) {
    use rand::Rng;
    let mut rng = rand::rng();

    let client_id = format!(
        "cobble-client-{}",
        (0..16)
            .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
            .collect::<String>()
    );

    let client_secret = (0..64)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect();

    (client_id, client_secret)
}
