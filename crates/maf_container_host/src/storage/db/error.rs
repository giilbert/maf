#[derive(Debug)]
pub enum TxnError {
    Anyhow { inner: anyhow::Error },
    DbErr { inner: sea_orm::DbErr },
}

impl std::error::Error for TxnError {}

impl std::fmt::Display for TxnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxnError::Anyhow { inner } => write!(f, "{}", inner),
            TxnError::DbErr { inner } => write!(f, "{}", inner),
        }
    }
}

impl From<anyhow::Error> for TxnError {
    fn from(err: anyhow::Error) -> Self {
        TxnError::Anyhow { inner: err }
    }
}

impl From<sea_orm::DbErr> for TxnError {
    fn from(err: sea_orm::DbErr) -> Self {
        TxnError::DbErr { inner: err }
    }
}
