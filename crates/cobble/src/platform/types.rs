#[derive(Debug, Clone)]
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("channel is closed")]
    Closed,
    #[error("buffer is full")]
    BufferFull,
    #[error("failed to serialize message")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("channel is closed")]
    Closed,
    #[error("channel is not ready")]
    NotReady,
    #[error("already listening")]
    AlreadyListening,
}
