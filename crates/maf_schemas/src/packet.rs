use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages sent from the client to the server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RxPacket {
    ChannelSend(ChannelSendRx),
    TypedRpcCall(TypedRpcRequestPacket),
}

/// Messages sent from the server to the client
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum TxPacket<'a, T> {
    Handshake(ServerHandshake),
    ChannelSend { channel: &'a str, data: &'a T },
    StoreUpdate(OneStoreUpdate<'a, T>),
    ManyStoreUpdate(Vec<OneStoreUpdate<'a, serde_json::Value>>),
    TypedRpcResponse(TypedRpcResponsePacket),
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerHandshake {
    pub id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TypedRpcRequestPacket {
    pub id: u32,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedRpcResponsePacket {
    pub id: u32,
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedRpcError {
    pub id: u32,
    pub code: String,
    pub error: String,
}

/// Represents either a borrowed or owned value.
#[derive(Debug)]
pub enum Borwned<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T: Serialize> Serialize for Borwned<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Borwned::Borrowed(value) => value.serialize(serializer),
            Borwned::Owned(value) => value.serialize(serializer),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OneStoreUpdate<'a, T> {
    pub store: &'a str,
    pub data: Borwned<'a, T>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChannelSendRx {
    pub channel: String,
    pub data: serde_json::Value,
}
