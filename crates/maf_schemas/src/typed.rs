use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSchema {
    pub stores: Vec<StoreSerialized>,
    pub rpcs: Vec<RpcSerialized>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreSerialized {
    pub name: String,
    pub select: TypeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RpcSerialized {
    pub name: String,
    pub params: Option<TypeKind>,
    pub result: Option<TypeKind>,
}
