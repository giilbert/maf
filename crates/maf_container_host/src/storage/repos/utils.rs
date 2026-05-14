use sea_orm::{DbErr, RuntimeErr, SqlxError, TransactionError};

use crate::storage::db::TxnError;

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
        is_db_error_code(self, "23505")
    }
}

impl DbErrorExt for TxnError {
    fn is_unique_violation(&self) -> bool {
        match self {
            TxnError::DbErr { inner } => inner.is_unique_violation(),
            _ => false,
        }
    }
}

impl DbErrorExt for TransactionError<TxnError> {
    fn is_unique_violation(&self) -> bool {
        match self {
            TransactionError::Connection(err) => err.is_unique_violation(),
            TransactionError::Transaction(err) => err.is_unique_violation(),
        }
    }
}
