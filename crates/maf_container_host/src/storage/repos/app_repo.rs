use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::storage::db::{app, org};

fn assert_related_org_exists(
    record: Option<(app::Model, Option<org::Model>)>,
) -> Result<Option<(app::Model, org::Model)>, DbErr> {
    match record {
        Some((app_model, Some(org_model))) => Ok(Some((app_model, org_model))),
        Some((_, None)) => Err(DbErr::RecordNotFound(
            "Associated organization not found".to_string(),
        )),
        None => Ok(None),
    }
}

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

pub async fn get_app_by_name_and_org_slug(
    conn: &impl ConnectionTrait,
    app_name: &str,
    org_slug: &str,
) -> Result<Option<app::Model>, DbErr> {
    let app = app::Entity::find()
        .left_join(org::Entity)
        .filter(app::Column::Name.eq(app_name))
        .filter(org::Column::Slug.eq(org_slug))
        .one(conn)
        .await?;

    Ok(app)
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

pub async fn get_app_by_name_and_org_slug_with_org(
    conn: &impl ConnectionTrait,
    app_name: &str,
    org_slug: &str,
) -> Result<Option<(app::Model, org::Model)>, DbErr> {
    let app = app::Entity::find()
        .find_also_related(org::Entity)
        .filter(app::Column::Name.eq(app_name))
        .filter(org::Column::Slug.eq(org_slug))
        .one(conn)
        .await?;

    assert_related_org_exists(app)
}

pub async fn get_app_by_service_account(
    conn: &impl ConnectionTrait,
    org_slug: &str,
    client_id: &str,
    secret: &str,
) -> Result<Option<(app::Model, org::Model)>, DbErr> {
    let app = app::Entity::find()
        .find_also_related(org::Entity)
        .filter(app::Column::ApiClientId.eq(client_id))
        .filter(app::Column::ApiSecret.eq(secret))
        .filter(org::Column::Slug.eq(org_slug))
        .one(conn)
        .await?;

    assert_related_org_exists(app)
}
