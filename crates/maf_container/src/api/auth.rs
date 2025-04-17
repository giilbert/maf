use sea_orm::{ColumnTrait, DbErr, EntityTrait, QueryFilter};

use crate::storage::db::{token, user};

use super::state::AppState;

impl AppState {
    pub async fn get_token_user(&self, token: &str) -> Result<Option<user::Model>, DbErr> {
        Ok(user::Entity::find()
            .left_join(token::Entity)
            .filter(token::Column::Token.eq(token))
            .one(&self.db)
            .await?)
    }
}
