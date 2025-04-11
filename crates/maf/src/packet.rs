use serde::{Deserialize, Serialize};

/// Messages sent from the client to the server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RxPacket {
    ChannelSend(ChannelSendRx),
}

/// Messages sent from the server to the client
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum TxPacket<'a, T> {
    ChannelSend { channel: &'a str, data: &'a T },
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChannelSendRx {
    pub channel: String,
    pub data: serde_json::Value,
}
