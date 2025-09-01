use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages sent from the client to the server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RxPacket {
    /// Send a message to a channel to be received by the server
    ChannelSend(ChannelSendRx),
    /// Invoke a server-side RPC method with parameters
    TypedRpcCall(TypedRpcRequestPacket),
}

/// Messages sent from the server to the client.
///
/// The generic parameter `T` and the lifetime `'a` are used to allow for zero-copy serialization
/// of data in some variants, while still allowing owned data in others. It is optional to use them,
/// with `T` defaulting to `()` and `'a` being inferred.
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum TxPacket<'a, T = ()> {
    /// Ready the client with server-side information about the connection
    Handshake(ServerHandshake),
    /// Send a message to a channel to be received by the client
    ChannelSend { channel: &'a str, data: &'a T },
    /// Update a single store with data
    StoreUpdate(OneStoreUpdate<'a, T>),
    /// Update multiple stores with data
    ManyStoreUpdate(Vec<OneStoreUpdate<'a, serde_json::Value>>),
    /// Respond to a previously invoked RPC call with the result
    TypedRpcResponse(TypedRpcResponsePacket),
}

/// Server: Ready the client with server-side information about the connection
#[derive(Debug, Clone, Serialize)]
pub struct ServerHandshake {
    pub id: Uuid,
}

/// Client: Invoke a server-side RPC method with parameters
#[derive(Debug, Clone, Deserialize)]
pub struct TypedRpcRequestPacket {
    /// Unique ID for this RPC call, used to match responses
    pub id: u32,
    pub method: String,
    pub params: serde_json::Value,
}

/// [Server] Respond to a previously invoked RPC call with the result
#[derive(Debug, Clone, Serialize)]
pub struct TypedRpcResponsePacket {
    pub id: u32,
    pub result: serde_json::Value,
}

/// Represents either a borrowed or owned value and **can only be serialized**.
///
/// This is different from `Cow` in that it does not require `T: ToOwned`, making it easier to use
/// with types that need to be serialized but do not implement `ToOwned`.
#[derive(Debug)]
pub enum Bull<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T: Serialize> Serialize for Bull<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Bull::Borrowed(value) => value.serialize(serializer),
            Bull::Owned(value) => value.serialize(serializer),
        }
    }
}

/// Server: Update a single store with data
#[derive(Debug, Serialize)]
pub struct OneStoreUpdate<'a, T> {
    pub store: &'a str,
    pub data: Bull<'a, T>,
}

/// Client: Send a message to a channel to be received by the server
#[derive(Debug, Deserialize, Clone)]
pub struct ChannelSendRx {
    pub channel: String,
    pub data: serde_json::Value,
}
