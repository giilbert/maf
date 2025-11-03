#[derive(Debug)]
pub struct TxnError {
    inner: anyhow::Error,
}

impl std::error::Error for TxnError {}

impl std::fmt::Display for TxnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl From<anyhow::Error> for TxnError {
    fn from(err: anyhow::Error) -> Self {
        TxnError { inner: err }
    }
}

impl From<sea_orm::DbErr> for TxnError {
    fn from(err: sea_orm::DbErr) -> Self {
        TxnError {
            inner: anyhow::anyhow!(err),
        }
    }
}
