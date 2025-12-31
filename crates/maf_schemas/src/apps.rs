use std::collections::{BTreeMap, HashMap};

use colored::Colorize;
use rand::Rng as _;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserAppRequest {
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

impl<S1, S2> PartialEq<(S1, S2)> for AppNameAndOrgSlug
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    fn eq(&self, other: &(S1, S2)) -> bool {
        self.app == other.0.as_ref() && self.org == other.1.as_ref()
    }
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
    /// A secret used for signing and verifying JWT payloads.
    pub secret: String,
    pub meta: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoomOptions {
    /// A key used to identify the room, defaults to the room ID or "default" for autocreated rooms.
    /// The key cannot be a UUID or "default" as they are reserved.
    pub key: Option<String>,
    /// Initial meta entries for the room.
    /// See https://maf.gilbertz.me/docs/build/meta for more information.
    pub meta: Option<MetaEntryMap>,
}

/// An instance of an application returned from the Platform API.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct App {
    pub id: Uuid,
    pub name: String,
    pub config: Option<String>,
    pub api_client_id: String,
    pub api_secret: String,
}

#[derive(Serialize)]
pub struct InfoResponse {
    /// A map of meta keys to their corresponding values. A [`BTreeMap`] is used here to ensure
    /// consistent ordering of keys in the serialized output.
    pub meta: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetaVisibility {
    /// Public metadata can be accessed by anyone, including clients.
    Public,
    /// Private metadata can only be accessed by the app (running on MAF Platform and service
    /// accounts) itself. Private metadata also includes all public metadata.
    Private,
}

impl MetaVisibility {
    /// Check if `self` can access metadata with the given `visibility`.
    pub fn can_access(self, visibility: MetaVisibility) -> bool {
        match (self, visibility) {
            (MetaVisibility::Private, _) => true,
            (MetaVisibility::Public, MetaVisibility::Public) => true,
            (MetaVisibility::Public, MetaVisibility::Private) => false,
        }
    }
}

/// An entry in the MAF Meta API.
///
/// The value stored in the entry should be unmarshalled from JSON using [`MetaEntry::deserialize`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntry {
    pub visibility: MetaVisibility,
    pub value: String,
}

impl MetaEntry {
    /// Deserialize the value of the meta entry into the specified type.
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.value)
    }

    /// Get the visibility of the meta entry.
    pub fn visibility(&self) -> &MetaVisibility {
        &self.visibility
    }
}

/// A meta entry stored as JSON value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMetaEntry {
    pub visibility: MetaVisibility,
    pub value: serde_json::Value,
}

impl JsonMetaEntry {
    pub fn serialize(&self) -> Result<MetaEntry, serde_json::Error> {
        Ok(MetaEntry {
            visibility: self.visibility,
            value: serde_json::to_string(&self.value)?,
        })
    }
}

pub type MetaEntryMap = HashMap<String, JsonMetaEntry>;
