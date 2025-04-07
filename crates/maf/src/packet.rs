use serde::{Deserialize, Serialize};

/// Messages sent from the client to the server
#[derive(Deserialize)]
pub enum RxPacket {}

/// Messages sent from the server to the client
#[derive(Serialize)]
pub enum TxPacket<'a, T> {
    ChannelSend { channel: &'a str, data: &'a T },
}
