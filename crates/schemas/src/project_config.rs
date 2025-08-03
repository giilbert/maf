use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::apps::RoomCreationStrategy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfigFile {
    pub name: String,
    #[serde(default = "default_room_creation_strategy")]
    pub rooms: RoomCreationStrategy,
    pub typed: Option<TypedConfig>,
    pub debug: TargetConfig,
    pub release: TargetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedConfig {
    pub out: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    pub command: String,
    pub output: String,
}

fn default_room_creation_strategy() -> RoomCreationStrategy {
    println!("default called");
    RoomCreationStrategy::AuthenticatedApiRequest
}
