use serde::{Deserialize, Serialize};

use crate::rpc::models::{TypedRpcRequestPacket, TypedRpcResponsePacket};

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
    ChannelSend { channel: &'a str, data: &'a T },
    StoreUpdate(OneStoreUpdate<'a, T>),
    ManyStoreUpdate(Vec<OneStoreUpdate<'a, serde_json::Value>>),
    TypedRpcResponse(TypedRpcResponsePacket),
}

#[derive(Debug, Serialize)]
pub struct OneStoreUpdate<'a, T> {
    pub store: &'a str,
    pub data: &'a T,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChannelSendRx {
    pub channel: String,
    pub data: serde_json::Value,
}
