use serde::{Deserialize, Serialize};

/// Messages sent from the client to the server
#[derive(Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RxPacket {}

/// Messages sent from the server to the client
#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum TxPacket<'a, T> {
    ChannelSend { channel: &'a str, data: &'a T },
}
