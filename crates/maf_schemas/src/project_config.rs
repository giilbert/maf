use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::apps::RoomCreationStrategy;

/// The configuration file for a MAF project, stored at `maf-project.toml` in the root of the
/// project directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfigFile {
    pub name: String,
    #[serde(default = "default_room_creation_strategy")]
    pub rooms: RoomCreationStrategy,
    pub typed: Option<TypedConfig>,
    pub debug: Option<TargetConfig>,
    pub release: Option<TargetConfig>,
    pub auth: Option<AuthConfig>,
}

impl ProjectConfigFile {
    pub fn validate(&self) -> Result<(), String> {
        if self.rooms == RoomCreationStrategy::AutoCreate && self.auth.is_some() {
            return Err("'auth' cannot be set when 'rooms' is 'AutoCreate'".into());
        }

        Ok(())
    }

    /// Creates a default ProjectConfigFile with the given name and default values for other fields.
    pub fn default_for(name: impl AsRef<str>) -> ProjectConfigFile {
        ProjectConfigFile {
            name: name.as_ref().to_string(),
            rooms: RoomCreationStrategy::AuthenticatedApiRequest,
            typed: None,
            debug: None,
            release: None,
            auth: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub mode: AuthMode,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AuthMode {
    Jwt,
}

impl AuthMode {
    pub fn format_with_description(&self) -> String {
        match self {
            AuthMode::Jwt => "Jwt (Your server needs to create and sign a JWT)".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    TypeScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedConfig {
    pub language: Language,
    pub out: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    pub command: String,
    pub output: String,
}

fn default_room_creation_strategy() -> RoomCreationStrategy {
    RoomCreationStrategy::AuthenticatedApiRequest
}
