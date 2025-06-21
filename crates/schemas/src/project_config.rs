use serde::{Deserialize, Serialize};

use crate::apps::RoomCreationStrategy;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfigFile {
    pub name: String,
    #[serde(default = "default_room_creation_strategy")]
    pub rooms: RoomCreationStrategy,

    pub debug: TargetConfig,
    pub release: TargetConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetConfig {
    pub command: String,
    pub output: String,
}

fn default_room_creation_strategy() -> RoomCreationStrategy {
    RoomCreationStrategy::AuthenticatedApiRequest
}
