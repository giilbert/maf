use anyhow::Context;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::storage::db::{org, org_member};

pub async fn create_app(
    conn: &impl ConnectionTrait,
    app: org::ActiveModel,
) -> anyhow::Result<org::Model> {
    let model = app.insert(conn).await.context("failed to create org")?;
    Ok(model)
}

pub async fn get_default_org_of_user(
    conn: &impl ConnectionTrait,
    user_id: Uuid,
) -> anyhow::Result<Option<org::Model>> {
    let org = org::Entity::find()
        .left_join(org_member::Entity)
        .filter(org::Column::IsDefault.eq(true))
        .filter(org_member::Column::Role.eq("OWNER"))
        .filter(org_member::Column::UserId.eq(user_id))
        .one(conn)
        .await
        .context("failed to get default org")?;

    Ok(org)
}
