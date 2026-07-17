use std::collections::{HashMap, HashSet};

use maf_schemas::apps::{
    AppNameAndOrgSlug, MetaEntryMap, RoomCreationStrategy, RoomId, RoomKey, RoomKeyHash,
};
use tokio::sync::RwLock;

use crate::ContainerResourceLimit;
use crate::server::app::App;
use crate::server::{CreateRoomCoreOptions, RoomCore, RoomHostImpl};

/// A struct representing all rooms on the server, allowing for lookup and management of rooms.
///
/// For users, this should be accessed through [`RoomHostImpl`].
///
/// In order to avoid deadlocks, the locks in this struct should always be acquired/released in the
/// order they are declared in this struct:
/// - `rooms`
/// - `auto_created_rooms`
/// - `api_created_rooms`
/// - `keys_to_rooms`
/// - `app_to_rooms`
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
    /// Maps an app to the set of rooms that were created for it. Includes both rooms that were
    /// created through the MAF Platform API and rooms that were automatically created for the app.
    app_to_rooms: RwLock<HashMap<AppNameAndOrgSlug, HashSet<RoomId>>>,
}

#[derive(Debug)]
pub struct CreateRoomOptions<'a> {
    /// The app which this room belongs to.
    pub app: &'a App,
    pub creation_strategy: RoomCreationStrategy,
    /// If the room creation strategy is [`RoomCreationStrategy::AuthenticatedApiRequest`], this is
    /// the key that the service client specified for the room. If a custom room key is not
    /// specified, the room ID will be used as the only key.
    ///
    /// If the room creation strategy is [`RoomCreationStrategy::AutoCreate`], this should be
    /// `None`, and the room will be created with [`RoomKey::Default`] as its only key.
    pub room_key: Option<String>,
    /// Optional meta information to be stored in the room's meta storage.
    pub meta: Option<MetaEntryMap>,
}

impl<R: RoomHostImpl> RoomsStorage<R> {
    /// Gets all rooms associated with the given app.
    pub async fn get_rooms_for_app(&self, app: &App) -> Vec<RoomCore<R>> {
        match self
            .app_to_rooms
            .read()
            .await
            .get(&app.app_name_and_org_slug())
        {
            Some(room_ids) => {
                let rooms = self.rooms.read().await;
                room_ids
                    .iter()
                    .filter_map(|id| rooms.get(id).cloned())
                    .collect()
            }
            None => {
                // No rooms for this app, return an empty vector.
                vec![]
            }
        }
    }

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
        // TODO: check if a room with the given key already exists for the app. If it does, return
        // an error. also prevent race conditions while creating a room with the same key for the
        // same app.
        // TODO: ensure no ugly race condition exists here since we don't hold all the locks while
        // creating the room

        // Keys not including the default key that is generated from the room ID.
        let extra_keys = match options.creation_strategy {
            RoomCreationStrategy::AutoCreate => vec![RoomKey::Default],
            RoomCreationStrategy::AuthenticatedApiRequest => options
                .room_key
                .clone()
                .map_or(vec![], |key| vec![RoomKey::Custom(key)]),
        };

        // The bundle contains the app's code and resources that will be used to run the room's
        // container.
        let bundle = self.host.load_bundle_for_app(options.app).await?;

        let (room_core, mut container) = RoomCore::new(
            self.host.clone(),
            CreateRoomCoreOptions {
                bundle,
                meta: options.meta,
                app: options.app.app_name_and_org_slug(),
                // TODO: make this configurable based on the app's config
                resource_limit: ContainerResourceLimit::small_defaults(),
                extra_keys,
            },
        )
        .await?;

        // TODO: move container run logic into RoomCore?
        // TODO: error handling for container run errors. if the container fails to start, we should
        // remove the room from storage and return an error.
        let id = room_core.id();
        tokio::spawn(async move {
            if let Err(error) = container.run().await {
                tracing::error!(?error, ?id, "room container run failed");
            }
        });

        // Update metadata about the room in the storage maps.
        self.rooms
            .write()
            .await
            .insert(room_core.id(), room_core.clone());

        let app_id = options.app.app_name_and_org_slug();
        self.app_to_rooms
            .write()
            .await
            .entry(options.app.app_name_and_org_slug())
            .or_default()
            .insert(room_core.id());

        // Some mappings depend on the room creation strategy, so we need to handle them separately.
        match options.creation_strategy {
            RoomCreationStrategy::AutoCreate => {
                let mut auto_created_rooms = self.auto_created_rooms.write().await;
                auto_created_rooms.insert(app_id.clone(), room_core.id());
            }
            RoomCreationStrategy::AuthenticatedApiRequest => {
                let mut api_created_rooms = self.api_created_rooms.write().await;
                api_created_rooms
                    .entry(app_id.clone())
                    .or_default()
                    .insert(room_core.id());
            }
        }

        // Add the room to the keys_to_rooms map for each of its keys.
        {
            let mut keys_to_rooms = self.keys_to_rooms.write().await;
            for key in room_core.keys() {
                keys_to_rooms.insert(
                    RoomKeyHash {
                        app: app_id.clone(),
                        key: key.clone(),
                    },
                    room_core.id(),
                );
            }
        }

        Ok(room_core)
    }

    /// Removes a room by its ID. This is used when a room is shut down and should not accept any
    /// newer connections or be returned in lookups.
    ///
    /// FIXME: is there a race condition with autocreated rooms here?
    pub async fn remove(&self, room_id: &RoomId) -> Option<RoomCore<R>> {
        let mut rooms = self.rooms.write().await;

        let mut keys_to_rooms = self.keys_to_rooms.write().await;
        let mut auto_created_rooms = self.auto_created_rooms.write().await;
        let mut api_created_rooms = self.api_created_rooms.write().await;
        let mut app_to_rooms = self.app_to_rooms.write().await;

        // Remove the room from the app_to_rooms map.
        let room = match rooms.remove(room_id) {
            Some(room) => room,
            None => return None,
        };

        // Remove the room from the app_to_rooms map.
        app_to_rooms
            .entry(room.app().clone())
            .or_default()
            .remove(room_id);

        // Remove the room from the auto_created_rooms map if it was auto-created.
        auto_created_rooms.remove(room.app());

        // Remove the room from the api_created_rooms map if it was created through the API.
        if let Some(api_created_room_ids) = api_created_rooms.get_mut(room.app()) {
            api_created_room_ids.remove(room_id);
        }

        // Remove the room from the keys_to_rooms map.
        keys_to_rooms.retain(|_, v| v != room_id);

        Some(room)
    }

    pub async fn get_autocreated_room(&self, app: &AppNameAndOrgSlug) -> Option<RoomCore<R>> {
        let rooms = self.rooms.read().await;
        let auto_created_rooms = self.auto_created_rooms.read().await;

        auto_created_rooms
            .get(app)
            .and_then(|room_id| rooms.get(room_id).cloned())
    }
}
