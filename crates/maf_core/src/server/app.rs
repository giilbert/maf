use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use maf_schemas::ErrorResponse;
use maf_schemas::apps::{AppNameAndOrgSlug, RoomKey, RoomKeyHash};
use maf_schemas::project_config::ProjectConfigFile;

use crate::server::RoomHostImpl;
use crate::server::types::AppOrgPath;

/// Information about an application, a bundled set of code and resources that can be used to create
/// MAF rooms.
///
/// This is the internal representation of an app, with more functionality than the serialized
/// representation [`maf_schemas::apps::App`].
///
/// This struct is also an [`axum::extract::FromRequestParts`] extractor, allowing it to be used in
/// route handlers to automatically look up an app by its name and organization slug from the
/// request path. If the app does not exist, the extractor will return a 404 error response.
///
/// TODO: make sure this isn't getting loaded multiple times per request
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
    pub fn room_key_hash(&self, room_key: RoomKey) -> RoomKeyHash {
        // XXX: less clones
        RoomKeyHash {
            app: self.app_name_and_org_slug(),
            key: room_key,
        }
    }

    pub fn parse_room_key(&self, room_key: &str) -> Result<RoomKey, ErrorResponse> {
        RoomKey::new(room_key, self.config.rooms).ok_or_else(|| {
            ErrorResponse::bad_request(Some(
                "invalid room key for the app's room creation strategy",
            ))
        })
    }
}

impl<R> FromRequestParts<R> for App
where
    R: RoomHostImpl,
{
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, state: &R) -> Result<Self, Self::Rejection> {
        let path = Path::<AppOrgPath>::from_request_parts(parts, state)
            .await
            .inspect_err(|err| {
                tracing::error!(
                    error=?err,
                    "the App extractor should only be used on routes with a org_slug and app_name parameter"
                );
            })
            .map_err(|_| ErrorResponse::internal_server_error(None))?;

        let app = state
            .app(path.app_org())
            .await
            .map_err(|err| {
                tracing::error!(error=?err, "failed to look up app");
                ErrorResponse::internal_server_error(None)
            })?
            .ok_or_else(|| ErrorResponse::not_found(Some("App not found.")))?;

        Ok(app)
    }
}
