use sea_orm::{DbErr, RuntimeErr, SqlxError};

pub fn is_db_error_code(err: &DbErr, error: impl AsRef<str>) -> bool {
    match err {
        DbErr::Query(RuntimeErr::SqlxError(SqlxError::Database(err))) => {
            let pg_err = err.downcast_ref::<sea_orm::sqlx::postgres::PgDatabaseError>();
            pg_err.code() == error.as_ref()
        }
        _ => false,
    }
}

pub trait DbErrorExt {
    fn is_unique_violation(&self) -> bool;
}

impl DbErrorExt for DbErr {
    fn is_unique_violation(&self) -> bool {
        is_db_error_code(&self, "23505")
    }
}
