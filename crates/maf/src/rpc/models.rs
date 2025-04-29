use serde::{Deserialize, Serialize};

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
