use std::sync::Arc;

use schemars::Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSchema {
    pub stores: Vec<StoreSerialized>,
    pub rpcs: Vec<RpcSerialized>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreSerialized {
    pub name: String,
    pub select: Arc<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcSerialized {
    pub name: String,
    pub params: Option<Arc<Schema>>,
    pub result: Arc<Schema>,
}
