use anyhow::Context;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

use crate::storage::db::{org, org_member, token, user};

pub async fn get_token_user(
    conn: &impl ConnectionTrait,
    token: &str,
) -> anyhow::Result<Option<user::Model>> {
    Ok(user::Entity::find()
        .left_join(token::Entity)
        .filter(token::Column::Token.eq(token))
        .one(conn)
        .await?)
}

pub async fn create_user_with_default_org(
    conn: &impl ConnectionTrait,
    user: user::ActiveModel,
) -> anyhow::Result<(user::Model, org::Model)> {
    let user = user.insert(conn).await.context("failed to create user")?;

    let org = org::ActiveModel {
        id: Set(user.id),
        slug: Set(user.username.clone()),
        name: Set(user.name.clone()),
        is_default: Set(true),
    }
    .insert(conn)
    .await?;

    let _org_member = org_member::ActiveModel {
        user_id: Set(user.id),
        org_id: Set(org.id),
        role: Set("OWNER".to_string()),
    }
    .insert(conn)
    .await
    .context("failed to create org member")?;

    Ok((user, org))
}

pub async fn internal_get_all_users(
    conn: &impl ConnectionTrait,
) -> anyhow::Result<Vec<user::Model>> {
    let users = user::Entity::find().all(conn).await?;
    Ok(users)
}
