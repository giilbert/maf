use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::runtime::wasi::bindings;

#[async_trait]
pub trait Connection: Send + Sync + 'static {
    fn id(&self) -> Uuid;

    fn send(&mut self, message: bindings::Message) -> Result<(), bindings::SendError>;

    fn disconnect(&mut self) -> Result<(), bindings::SendError>;

    /// Returns the authentication data associated with this connection, if any.
    fn auth(&self) -> Option<&serde_json::Value>;

    async fn get_message_channel(&self) -> anyhow::Result<mpsc::Receiver<bindings::Message>>;
}

pub type BoxedConnection = Box<dyn Connection>;
