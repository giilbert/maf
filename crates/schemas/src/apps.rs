use colored::Colorize;
use serde::{Deserialize, Serialize};

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
