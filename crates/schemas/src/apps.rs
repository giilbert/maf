use colored::Colorize;
use rand::Rng as _;
use serde::{Deserialize, Serialize};

pub fn generate_room_secret() -> String {
    let mut rng = rand::rng();

    (0..64)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAppConfig {
    pub rooms: RoomCreationStrategy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserAppRequest {
    pub name: String,
    pub config: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomCreationStrategy {
    /// Auto-create a room and put everyone in it
    AutoCreate,
    /// Rooms are created by an authenticated API request
    AuthenticatedApiRequest,
}

impl RoomCreationStrategy {
    pub fn format(&self) -> String {
        match self {
            RoomCreationStrategy::AuthenticatedApiRequest => {
                "Authenticated API Request".to_string()
            }
            RoomCreationStrategy::AutoCreate => "Auto Create".to_string(),
        }
    }

    pub fn format_with_description(&self) -> String {
        match self {
            RoomCreationStrategy::AuthenticatedApiRequest => {
                format!(
                    "Authenticated API Request {}",
                    "(A server-side API call is used to create rooms)".dimmed()
                )
            }
            RoomCreationStrategy::AutoCreate => {
                format!(
                    "Auto Create {}",
                    "(Rooms are automatically created when a user joins; all users are placed in the same room)".dimmed()
                )
            }
        }
    }
}

pub type RoomId = uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppNameAndOrgSlug {
    pub app: String,
    pub org: String,
}

/// A struct used for hashing the room key and app name, used to quickly look up rooms by key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomKeyHash {
    pub app: AppNameAndOrgSlug,
    pub key: String,
}

/// Serialized information about a room, used for API responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomInfo {
    pub id: RoomId,
    pub key: String,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoomOptions {
    /// A key used to identify the room, defaults to the room ID or "default" for autocreated rooms.
    /// The key cannot be a UUID or "default" as they are reserved.
    pub key: Option<String>,
}
