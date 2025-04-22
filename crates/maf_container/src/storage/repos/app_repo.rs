use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::storage::db::app;

pub async fn create_app(
    conn: &impl ConnectionTrait,
    app: app::ActiveModel,
) -> Result<app::Model, DbErr> {
    let result = app.insert(conn).await?;
    Ok(result)
}

pub async fn get_apps_by_org_id(
    conn: &impl ConnectionTrait,
    org_id: Uuid,
) -> Result<Vec<app::Model>, DbErr> {
    let apps = app::Entity::find()
        .filter(app::Column::OrgId.eq(org_id))
        .all(conn)
        .await?;
    Ok(apps)
}

pub async fn get_app_by_name_and_org_id(
    conn: &impl ConnectionTrait,
    name: &str,
    org_id: Uuid,
) -> Result<Option<app::Model>, DbErr> {
    let app = app::Entity::find()
        .filter(app::Column::Name.eq(name))
        .filter(app::Column::OrgId.eq(org_id))
        .one(conn)
        .await?;

    Ok(app)
}
