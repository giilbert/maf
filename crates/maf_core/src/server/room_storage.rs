use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use maf_schemas::apps::{
    AppNameAndOrgSlug, MetaEntryMap, RoomCreationStrategy, RoomId, RoomKey, RoomKeyHash,
};
use tokio::sync::RwLock;

use crate::ContainerResourceLimit;
use crate::server::app::App;
use crate::server::{CreateRoomCoreOptions, RoomCore, RoomHostImpl};

/// A struct representing all rooms on the server, allowing for lookup and management of rooms.
///
/// Normally, this would be accessed through [`RoomHost`] or [`RoomHostImpl`].
#[derive(Debug, Default)]
pub struct RoomsStorage<R: RoomHostImpl> {
    host: R,

    /// All rooms on the server. Mapped by their unique [`RoomId`].
    rooms: RwLock<HashMap<RoomId, RoomCore<R>>>,

    /// Maps an app (identified by its name and org slug) to the room that was automatically created
    /// for it, if it was created with the [`RoomCreationStrategy::AutoCreate`] strategy.
    ///
    /// See [`RoomCreationStrategy::AutoCreate`] for more details.
    auto_created_rooms: RwLock<HashMap<AppNameAndOrgSlug, RoomId>>,
    /// Maps an app (identified by its [`AppNameAndOrgSlug`] to the set of rooms that were
    /// created for it through the MAF Platform API.
    ///
    /// See [`RoomCreationStrategy::AuthenticatedApiRequest`] for more details.
    api_created_rooms: RwLock<HashMap<AppNameAndOrgSlug, HashSet<RoomId>>>,
    /// Maps a app and a *room key* (a string identifier for a room that is chosen by the client)
    /// to the ID of the room that was created for it with that key. **The ID of a room is always a
    /// key to the room in this maps**.
    keys_to_rooms: RwLock<HashMap<RoomKeyHash, RoomId>>,
}

#[derive(Debug)]
pub struct CreateRoomOptions<'a> {
    /// The app which this room belongs to.
    pub app: &'a App,
    pub creation_strategy: RoomCreationStrategy,
    pub room_key: RoomKey,
    /// Optional meta information to be stored in the room's meta storage.
    pub meta: Option<MetaEntryMap>,
}

impl<R: RoomHostImpl> RoomsStorage<R> {
    /// Gets a room by its ID. Returns `None` if no room with the given ID exists.
    pub async fn get(&self, room_id: &RoomId) -> Option<RoomCore<R>> {
        let rooms = self.rooms.read().await;
        rooms.get(room_id).cloned()
    }

    /// Gets a room by its room key (a string identifier for a room that is chosen by the developer)
    /// or its ID. Returns `None` if no room with the given key or ID exists.
    pub async fn get_by_key(&self, app: &App, room_key: RoomKey) -> Option<RoomCore<R>> {
        let rooms = self.rooms.read().await;
        let keys_to_rooms = self.keys_to_rooms.read().await;

        keys_to_rooms
            .get(&app.room_key_hash(room_key))
            .and_then(|room_id| rooms.get(room_id).cloned())
    }

    /// Creates a room with the given options.
    pub async fn create(&self, options: CreateRoomOptions<'_>) -> anyhow::Result<RoomCore<R>> {
        let room = RoomCore::new(
            self.host.clone(),
            CreateRoomCoreOptions {
                bundle: self.host.load_bundle_for_app(options.app).await?,
                meta: options.meta,
                // TODO: make this configurable based on the app's config
                resource_limit: ContainerResourceLimit::small_defaults(),
            },
        );

        todo!();
    }

    /// Removes a room by its ID. This is used when a room is shut down and should not accept any
    /// newer connections or be returned in lookups.
    ///
    /// FIXME: is there a race condition with autocreated rooms here?
    pub async fn remove(&self, room_id: &RoomId) -> Option<RoomCore<R>> {
        let mut rooms = self.rooms.write().await;

        let mut auto_created_rooms = self.auto_created_rooms.write().await;
        let mut api_created_rooms = self.api_created_rooms.write().await;
        let mut keys_to_rooms = self.keys_to_rooms.write().await;

        rooms.remove(room_id);

        todo!();
    }

    pub async fn get_autocreated_room(&self, app: &AppNameAndOrgSlug) -> Option<RoomCore<R>> {
        let rooms = self.rooms.read().await;
        let auto_created_rooms = self.auto_created_rooms.read().await;

        auto_created_rooms
            .get(app)
            .and_then(|room_id| rooms.get(room_id).cloned())
    }
}
