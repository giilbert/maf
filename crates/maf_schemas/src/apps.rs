use std::collections::{BTreeMap, HashMap};

use colored::Colorize;
use rand::Rng as _;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generates a random string to use as a secret for signing JWTs for room authentication.
pub fn generate_room_secret() -> String {
    let mut rng = rand::rng();

    (0..256)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect()
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

/// A user-specified identifier for a room, used to look up rooms by a string.
///
/// [`RoomKey::Default`] is used for rooms that are created with
/// [`RoomCreationStrategy::AutoCreate`]. [`RoomKey::Custom`] is used for rooms that are created
/// with other methods, such as through the MAF Platform API.
///
/// Note that "default" is a reserved key and cannot be used for custom rooms.
///
/// When serializing/deserializing, this gets converted to a string, with "default" being used for
/// [`RoomKey::Default`] and the custom string being used for [`RoomKey::Custom`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoomKey {
    Default,
    Custom(String),
}

impl RoomKey {
    /// Creates a new [`RoomKey`] from the given string and room creation strategy.
    ///
    /// Returns `None` if the key is invalid (e.g., "default" for a custom room).
    pub fn new(key: &str, strategy: RoomCreationStrategy) -> Option<Self> {
        match strategy {
            RoomCreationStrategy::AutoCreate => Some(RoomKey::Default),
            RoomCreationStrategy::AuthenticatedApiRequest => {
                if key == "default" {
                    None
                } else {
                    Some(RoomKey::Custom(key.to_string()))
                }
            }
        }
    }
}

impl Serialize for RoomKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RoomKey::Default => serializer.serialize_str("default"),
            RoomKey::Custom(key) => serializer.serialize_str(key),
        }
    }
}

impl<'de> Deserialize<'de> for RoomKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "default" {
            Ok(RoomKey::Default)
        } else {
            Ok(RoomKey::Custom(s))
        }
    }
}

/// A struct used for hashing the room key and app name, used to quickly look up rooms by key.
///
/// TODO: use Cow<'a, str> or some kind of immutable string
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomKeyHash {
    pub app: AppNameAndOrgSlug,
    pub key: RoomKey,
}

/// Serialized information about a room **for service use**, used for API responses.
///
/// This is different from [`PublicRoomInfo`] which is used for client-facing API responses in that
/// this struct contains sensitive information that should not be exposed to clients, such as the
/// room secret.
///
/// Used by:
/// - GET `/api/v1/apps/{org_slug}/{app_name}/rooms`
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRoomInfo {
    pub id: RoomId,
    pub keys: Vec<RoomKey>,
    /// A secret used for signing and verifying JWT payloads.
    pub secret: String,
    pub meta: BTreeMap<String, serde_json::Value>,
}

pub const MAX_ROOM_KEY_LENGTH: usize = 128;

/// A request to create a room, used for API requests.
///
/// Used by:
/// - POST `/api/v1/apps/{org_slug}/{app_name}/rooms`
#[derive(Debug, Deserialize)]
pub struct ServiceCreateRoomOptions {
    /// An additional key used to identify the room. A default key being the room's ID will always
    /// be created for the room, but this allows for a custom key to be specified as well.
    pub key: Option<String>,
    /// Initial meta entries for the room.
    /// See https://maf.gilbertz.me/docs/build/meta for more information.
    pub meta: Option<MetaEntryMap>,
}

/// An instance of an application returned from the Platform API.
///
/// This is the serialized representation of an App! `maf_core` has
#[derive(Debug, Clone, serde::Deserialize)]
pub struct App {
    pub id: Uuid,
    pub name: String,
    pub config: Option<String>,
    pub api_client_id: String,
    pub api_secret: String,
}

/// Public information about a room.
///
/// Used by:
/// - GET `/@/{org_slug}/{app_name}/{room_key}`.
#[derive(Serialize)]
pub struct PublicRoomInfo {
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

/// Used by:
/// - GET `/@/{org_slug}/{app_name}/{room_key}/connect` route to parse query parameters.
#[derive(Deserialize)]
pub struct ConnectQueryParams {
    /// A JWT token for authenticating the user connecting to the room. This token is only needed
    /// if the auth.mode in config for the room requires it.
    pub token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RoomListQueryParams {
    pub by_key: Option<String>,
    pub by_id: Option<Uuid>,
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum RoomQueryResponse {
    /// A single room. This is returned when filtering by a specific key or ID.
    Single(ServiceRoomInfo),
    /// Multiple rooms. This is returned when no specific filter is applied.
    Multiple(Vec<ServiceRoomInfo>),
}
