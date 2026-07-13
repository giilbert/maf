use maf_schemas::apps::{AppNameAndOrgSlug, RoomKeyHash};
use maf_schemas::project_config::ProjectConfigFile;

/// Information about an application, a bundled set of code and resources that can be used to create
/// MAF rooms.
///
/// This is the internal representation of an app, with more functionality than the serialized
/// representation [`maf_schemas::apps::App`].
#[derive(Debug)]
pub struct App {
    name: String,
    org: String,
    config: ProjectConfigFile,
}

impl App {
    /// Creates a new [`App`] from the serialized representation of an app, parsing the config if it
    /// exists, or using a default config if it does not. If the config is invalid, this function
    /// will return the error from the TOML parser.
    pub fn from_serialized(
        name: impl AsRef<str>,
        org: impl AsRef<str>,
        serialized: maf_schemas::apps::App,
    ) -> Result<Self, toml::de::Error> {
        Ok(Self {
            name: name.as_ref().to_string(),
            org: org.as_ref().to_string(),
            config: serialized
                .config
                .map(|config| toml::from_str(&config))
                .transpose()?
                .unwrap_or_else(|| ProjectConfigFile::default_for(name)),
        })
    }

    /// Getter for the app's config, which is a parsed representation of the app's `config` field in
    /// the serialized representation of an app.
    pub fn config(&self) -> &ProjectConfigFile {
        &self.config
    }

    /// Getter for the app's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Getter for the app's organization.
    pub fn org(&self) -> &str {
        &self.org
    }

    /// Returns an [`AppNameAndOrgSlug`] for this app, which is used in various places as a unique
    /// identifier for an app.
    pub fn app_name_and_org_slug(&self) -> AppNameAndOrgSlug {
        // XXX: less clones
        AppNameAndOrgSlug {
            app: self.name.clone(),
            org: self.org.clone(),
        }
    }

    /// Returns a [`RoomKeyHash`] for the given room key, which can be used to look up a room by its
    /// key in the [`crate::server::room_storage::RoomsStorage`].
    pub fn room_hash_key(&self, room_key: &str) -> RoomKeyHash {
        // XXX: less clones
        RoomKeyHash {
            app: self.app_name_and_org_slug(),
            key: room_key.to_string(),
        }
    }
}
