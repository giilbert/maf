//! Core implementation of MAF rooms, which are the main unit of isolation and execution in MAF.
//!
//! When a client connects to a MAF server, they connect to a specific room. Each room has its own
//! set of connected clients, its own instance of the user code (the "container"), and its own
//! metadata.
//!
//! The [`RoomCore`] struct needs a reference to a [`RoomHost`] implementation, which is provided by
//! the server and allows the room to access external functionality, such as the container runtime.
//! The API is designed this way to allow the core room logic to be decoupled from the server
//! spawning and driving the rooms (specifically, `maf_platform_host` vs `maf_cli`).

use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Context;
use biscuit::jwa::SignatureAlgorithm;
use biscuit::jws::Secret;
use biscuit::{ClaimsSet, JWT};
use maf_schemas::apps::{
    AppNameAndOrgSlug, MetaEntryMap, MetaVisibility, RoomId, RoomKey, generate_room_secret,
};
use tokio::sync::oneshot;
use uuid::Uuid;

use super::Bundle;
use crate::container::meta::MetaStorage;
use crate::container::{ContainerHandle, ContainerResourceLimit, CreateContainerOptions};
use crate::server::app::App;
use crate::server::room_storage::RoomsStorage;
use crate::wasi::HookRequest;
use crate::wasi::bindings::{self, HookRequestCaller, HookRequestInit};
use crate::{Connection, Container, ContainerResourceStats, ContainerRuntime};

/// A trait representing external logic for managing a MAF room.
///
/// Implementors of this trait provide the core room logic with access to external functionality. As
/// such, implementors should have the following as fields and expose them through the trait's API:
/// - [`ContainerRuntime`]: for managing and running user code containers associated with rooms.
/// - [`RoomsStorage`]: for managing all rooms on the server, allowing for lookup and management.
///
/// See the [module level documentation](`crate::server::room`) for more details on the design and
/// intended usage of this trait and [`RoomCore`].
///
/// TODO: error types
pub trait RoomHostImpl: Debug + Clone + Send + Sync + 'static {
    /// Returns a reference to the container runtime that should be used to create the room's
    /// container.
    fn container_runtime(&self) -> &ContainerRuntime;

    /// Returns a reference to the [`RoomsStorage`], managing all rooms on the server.
    fn room_storage(&self) -> &RoomsStorage<Self>;

    // Development server vs. MAF Platform Host should implement the following methods very
    // differently. These methods involve authentication or some form of loading data.

    /// Updates the last activity timestamp for the server. This is used to determine when the
    /// server was last active, and can be used to shut down the server if it has been inactive for
    /// a long time.
    fn update_last_activity(&self) -> anyhow::Result<()>;

    /// Checks if the given API key is valid for the given app.
    ///
    /// Returns `Ok(true)` if the API key is valid, `Ok(false)` if the API key is invalid, and `Err`
    /// if there was an error during the validation process.
    fn validate_api_key(
        &self,
        app: &App,
        request: &axum::extract::Request,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;

    /// Looks up an app by its name and which organization it belongs to.
    ///
    /// Returns `Ok(None)` if the app does not exist, or `Err` if there was an error during the
    /// lookup.
    fn app(&self, id: AppNameAndOrgSlug) -> anyhow::Result<Option<App>>;

    /// Loads the bundle for the given app.
    fn load_bundle_for_app(&self, app: &App)
    -> impl Future<Output = anyhow::Result<Bundle>> + Send;
}

/// The core implementation of a MAF room, containing the container, bundle, and other internal
/// data. This struct is intended to be wrapped in some container that decorates it with additional
/// functionality, such as connection management, room lifecycle management, etc.
#[derive(Debug, Clone)]
pub struct RoomCore<R: RoomHostImpl> {
    inner: Arc<RoomCoreInner<R>>,
}

#[derive(Debug)]
struct RoomCoreInner<R: RoomHostImpl> {
    /// A reference to the host's (the server driving this room) API for managing the room.
    #[allow(dead_code)] // TODO: remove this if host is used
    host: R,
    /// The unique identifier of the room.
    id: Uuid,
    /// Which organization and app this room belongs to.
    app: AppNameAndOrgSlug,
    /// The keys associated with the room. The first key is always the default key generated from
    /// the room ID. Additional keys can be [`RoomKey::Default`] or [`RoomKey::Custom`].
    keys: Vec<RoomKey>,
    /// A secret associated with the room, used for signing JWTs for authentication.
    secret: String,
    /// A handle to the room's container, which can be used to interact with the running instance of
    /// user code associated with the room.
    container: ContainerHandle,
    /// The bundle associated with the room, which contains metadata and the WASM module needed to
    /// run the room's container.
    bundle: Bundle,
}

#[derive(Debug, Clone)]
pub struct CreateRoomCoreOptions {
    pub bundle: Bundle,
    pub resource_limit: ContainerResourceLimit,
    pub meta: Option<MetaEntryMap>,
    /// The app that this room belongs to.
    pub app: AppNameAndOrgSlug,
    /// The keys associated with the room, not including the default key that is generated from the
    /// room ID.
    pub extra_keys: Vec<RoomKey>,
}

impl<R: RoomHostImpl> RoomCore<R> {
    pub async fn new(host: R, options: CreateRoomCoreOptions) -> anyhow::Result<(Self, Container)> {
        let room_id = Uuid::new_v4();
        let secret = generate_room_secret();
        let container = Container::load_from_binary(
            host.container_runtime(),
            room_id,
            CreateContainerOptions {
                bytes: options.bundle.wasm_module_bytes(),
                resource_limit: options.resource_limit,
                meta: options.meta,
                secret: secret.clone(),
            },
        )
        .await?;

        let mut keys = vec![];
        keys.push(RoomKey::Custom(room_id.to_string()));
        keys.extend(options.extra_keys.clone());

        Ok((
            Self {
                inner: Arc::new(RoomCoreInner {
                    host,
                    id: room_id,
                    secret,
                    container: container.handle(),
                    bundle: options.bundle,
                    app: options.app,
                    keys,
                }),
            },
            container,
        ))
    }

    /// Returns the unique identifier of the room.
    pub fn id(&self) -> RoomId {
        self.inner.id
    }

    /// Returns the secret associated with the room, which is used for signing JWTs for
    /// authentication.
    ///
    /// Accessing the secret isn't super helpful for most use cases, so see
    /// [`RoomInner::decode_token`].
    pub fn secret(&self) -> &str {
        &self.inner.secret
    }

    /// Returns the bundle associated with the room, which contains metadata and the WASM module
    /// needed to run the room's container.
    pub fn bundle(&self) -> &Bundle {
        &self.inner.bundle
    }

    /// Returns the app that this room belongs to.
    pub fn app(&self) -> &AppNameAndOrgSlug {
        &self.inner.app
    }

    /// Returns the keys associated with the room. The first key is always the default key generated
    /// from the room ID. Additional keys can be [`RoomKey::Default`] or [`RoomKey::Custom`].
    pub fn keys(&self) -> &[RoomKey] {
        &self.inner.keys
    }

    /// Returns a reference to the room's container, which can be used to interact with the running
    /// instance of user code associated with the room.
    pub fn container(&self) -> &ContainerHandle {
        &self.inner.container
    }

    /// Returns a reference to the room's [`MetaStorage`], which can be used to read and write
    /// metadata (MAF service) entries associated with the room.
    pub fn meta_storage(&self) -> &MetaStorage {
        &self.inner.container.meta
    }

    /// Returns a struct containing information about the room's container's resource usage.
    pub fn resource_usage(&self) -> &ContainerResourceStats {
        &self.inner.container.resources
    }

    /// Adds a new connection to the room. The connection will be managed by the room and will be
    /// automatically closed when the room is closed.
    pub async fn add_connection(&self, connection: impl Connection) -> anyhow::Result<()> {
        match self.container().add_connection(Box::new(connection)).await {
            Ok(_) => tracing::info!("connection added to room {}", self.id()),
            Err(_) => anyhow::bail!("failed to add connection to room {}", self.id()),
        }

        Ok(())
    }

    /// Calls a hook in the room's container with the given method and data, and waits for a
    /// response. The caller parameter indicates the source of the hook call, which can be used by
    /// the container to implement different behavior based on the caller (for example, a hook call
    /// from MAF Platform vs. a hook call from a connected client).
    pub async fn call_hook(
        &self,
        caller: HookRequestCaller,
        method: String,
        data: bindings::HookBody,
    ) -> anyhow::Result<bindings::HookBody> {
        let (message_tx, message_rx) = oneshot::channel::<bindings::HookBody>();

        let request = HookRequest::new(
            HookRequestInit {
                caller,
                method,
                data,
            },
            message_tx,
        );
        self.container().send_hook_request(request).await?;

        match tokio::time::timeout(std::time::Duration::from_secs(5), message_rx).await? {
            Ok(response) => {
                tracing::info!("hook response: {:?}", response);
                Ok(response)
            }
            Err(_) => {
                tracing::info!("hook response timed out");
                anyhow::bail!("failed to receive hook response");
            }
        }
    }

    /// Decodes and verifies a JWT signed with the room's secret.
    ///
    /// If the token is valid, returns the decoded claims as a [`serde_json::Value`]. If the token
    /// is invalid or verification fails, returns `Err`.
    pub fn decode_token(&self, token: &str) -> Result<serde_json::Value, anyhow::Error> {
        let mut verified_jwt: JWT<ClaimsSet<serde_json::Value>, serde_json::Value> =
            JWT::new_encoded(token)
                .decode(
                    &Secret::bytes_from_str(self.secret()),
                    SignatureAlgorithm::HS256,
                )
                .context("failed to decode JWT")?;

        let payload = verified_jwt.payload_mut().expect("JWT should be decoded");

        // Check that the audience matches the room ID
        let audience = payload.registered.audience.clone();
        let room_id = self.id().to_string();
        if !audience.is_some_and(|aud| aud.contains(&room_id)) {
            anyhow::bail!("invalid audience in JWT");
        }

        serde_json::to_value(payload).context("failed to reencode JWT")
    }

    /// Returns a [`maf_schemas::apps::ServiceRoomInfo`] struct containing information about the
    /// room that is suitable for service accounts to manage the room.
    pub async fn service_room_info(&self) -> maf_schemas::apps::ServiceRoomInfo {
        maf_schemas::apps::ServiceRoomInfo {
            id: self.id(),
            keys: self.keys().to_vec(),
            meta: self
                .meta_storage()
                .list_values(MetaVisibility::Private)
                .await,
            secret: self.secret().to_string(),
        }
    }
}
