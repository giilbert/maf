use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use maf_schemas::apps::{
    AppNameAndOrgSlug, MetaEntryMap, RoomCreationStrategy, RoomId, RoomKey, RoomKeyAndApp,
};
use tokio::sync::{Mutex, Notify, RwLock, RwLockReadGuard};

use crate::ContainerResourceLimit;
use crate::server::app::App;
use crate::server::room::UpgradeableRoomHostImpl;
use crate::server::{CreateRoomCoreOptions, RoomCore, RoomHostImpl};

/// A struct representing all rooms on the server, allowing for lookup and management of rooms.
///
/// For users, this should be accessed through [`RoomHostImpl`].
///
/// In order to avoid deadlocks, the locks in this struct should always be acquired/released in the
/// order they are declared in this struct:
/// - `maps`
/// - `pending_room_creations`
///
/// ## Room Crashes
///
/// A room may crash if its container does something illegal (e.g. segfaults, runs out of memory,
/// panics, etc.). When a room crashes, it will be removed from the storage maps automatically and
/// it will no longer be returned in lookups. There is a race condition where a room crashes but is
/// still registered in the storage maps, so it is possible for that room to be returned when a
/// client tries to connect. In this case, the connection will fail and the client should retry the
/// connection. **In cases where the room crashes, there is no guarantee that the client will be
/// able to connect to the room successfully.**
#[derive(Debug, Clone)]
pub struct RoomsStorage<R: RoomHostImpl>(Arc<RoomStorageInner<R>>);

#[derive(Debug)]
struct RoomStorageInner<R: RoomHostImpl> {
    /// A handle to access host APIs.
    ///
    /// This is an Option because the host may not be available when the storage is created, but
    /// will be set later when the host is available (the host needs the storage to be created
    /// first). See [`RoomsStorage::set_host`] and [`RoomsStorage::host`].
    host: R::WeakRef,

    maps: RwLock<RoomMaps<R>>,

    /// A map of (rooms keys, apps) to signals for rooms that are currently being created. This is
    /// used to prevent race conditions when creating rooms with the same key for the same app.
    ///
    /// If a room is being created for a given key and app (tuples of keys and apps need to be
    /// unique, so this is a unique identifier for a room), any other calls to create a room with
    /// the same key and app will wait for the first call to finish and then return the same room.
    /// The [`Notify`] is used to signal the second-creator when the room has been created and is
    /// ready to be returned.
    ///
    /// Locks held on this mutex should be super short-lived as it is shared across everyone.
    pending_room_creations: Mutex<HashMap<RoomKeyAndApp, Arc<Notify>>>,
}

#[derive(Debug)]
struct RoomMaps<R: RoomHostImpl> {
    /// All rooms on the server. Mapped by their unique [`RoomId`].
    rooms: HashMap<RoomId, RoomCore<R>>,
    /// Maps an app (identified by its name and org slug) to the room that was automatically created
    /// for it, if it was created with the [`RoomCreationStrategy::AutoCreate`] strategy.
    ///
    /// See [`RoomCreationStrategy::AutoCreate`] for more details.
    auto_created_rooms: HashMap<AppNameAndOrgSlug, RoomId>,
    /// Maps an app (identified by its [`AppNameAndOrgSlug`] to the set of rooms that were
    /// created for it through the MAF Platform API.
    ///
    /// See [`RoomCreationStrategy::AuthenticatedApiRequest`] for more details.
    api_created_rooms: HashMap<AppNameAndOrgSlug, HashSet<RoomId>>,
    /// Maps a app and a *room key* (a string identifier for a room that is chosen by the client)
    /// to the ID of the room that was created for it with that key. **The ID of a room is always a
    /// key to the room in this maps**.
    keys_to_rooms: HashMap<RoomKeyAndApp, RoomId>,
    /// Maps an app to the set of rooms that were created for it. Includes both rooms that were
    /// created through the MAF Platform API and rooms that were automatically created for the app.
    app_to_rooms: HashMap<AppNameAndOrgSlug, HashSet<RoomId>>,
}

impl<R: RoomHostImpl> RoomMaps<R> {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            auto_created_rooms: HashMap::new(),
            api_created_rooms: HashMap::new(),
            keys_to_rooms: HashMap::new(),
            app_to_rooms: HashMap::new(),
        }
    }
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
    pub fn new(host: R::WeakRef) -> Self {
        Self(Arc::new(RoomStorageInner {
            host,
            maps: RwLock::new(RoomMaps::new()),
            pending_room_creations: Mutex::new(HashMap::new()),
        }))
    }

    /// Gets a strong reference to the host, if it is still available. Returns an error if the host
    /// is no longer available (i.e. the weak reference has been dropped).
    pub fn host(&self) -> anyhow::Result<R> {
        self.0.host.upgrade().context("host is no longer available")
    }

    /// Getter for the `rooms` map. This is used for testing and debugging purposes.
    pub async fn rooms_map(&self) -> RwLockReadGuard<'_, HashMap<RoomId, RoomCore<R>>> {
        RwLockReadGuard::map(self.0.maps.read().await, |maps| &maps.rooms)
    }

    /// Getter for the `auto_created_rooms` map. This is used for testing and debugging purposes.
    pub async fn auto_created_rooms_map(
        &self,
    ) -> RwLockReadGuard<'_, HashMap<AppNameAndOrgSlug, RoomId>> {
        RwLockReadGuard::map(self.0.maps.read().await, |maps| &maps.auto_created_rooms)
    }

    /// Getter for the `api_created_rooms` map. This is used for testing and debugging purposes.
    pub async fn api_created_rooms_map(
        &self,
    ) -> RwLockReadGuard<'_, HashMap<AppNameAndOrgSlug, HashSet<RoomId>>> {
        RwLockReadGuard::map(self.0.maps.read().await, |maps| &maps.api_created_rooms)
    }

    /// Gets all rooms associated with the given app.
    pub async fn get_rooms_for_app(&self, app: &App) -> Vec<RoomCore<R>> {
        let maps = self.0.maps.read().await;
        maps.app_to_rooms
            .get(&app.app_name_and_org_slug())
            .map(|room_ids| {
                room_ids
                    .iter()
                    .filter_map(|id| maps.rooms.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets a room by its ID. Returns `None` if no room with the given ID exists.
    pub async fn get(&self, room_id: &RoomId) -> Option<RoomCore<R>> {
        let maps = self.0.maps.read().await;
        maps.rooms
            .get(room_id)
            .cloned()
            .inspect(|r| r.container().mark_activity())
    }

    /// Gets a room by its room key (a string identifier for a room that is chosen by the developer)
    /// or its ID. Returns `None` if no room with the given key or ID exists.
    pub async fn get_by_key(&self, app: &App, room_key: RoomKey) -> Option<RoomCore<R>> {
        let maps = self.0.maps.read().await;
        let rooms = &maps.rooms;

        maps.keys_to_rooms
            .get(&app.room_key_hash(room_key))
            .and_then(|room_id| rooms.get(room_id).cloned())
            .inspect(|r| r.container().mark_activity())
    }

    /// Creates a room with the given options.
    ///
    /// ## Race Behavior With Room Creation
    /// If the room creation strategy is [`RoomCreationStrategy::AutoCreate`] and two rooms need to
    /// be created for the same app at the same time (e.g. one call to this function is made while
    /// another call is still in progress), the second call will wait for the first call to finish
    /// and then return the same room.
    ///
    /// If the room creation strategy is anything else, the second call will return an error since
    /// the room key is already in use.
    pub async fn create(&self, options: CreateRoomOptions<'_>) -> anyhow::Result<RoomCore<R>> {
        let check_pending_room = match options.creation_strategy {
            // Always check for pending rooms with the default (auto-created) key since there can
            // only be one auto-created room per app if the room creation strategy is AutoCreate.
            RoomCreationStrategy::AutoCreate => Some(
                options
                    .app
                    .app_name_and_org_slug()
                    .with_room_key(RoomKey::Default),
            ),
            // If someone creates a room through the API, we only check if rooms with the same key
            // are being created at the same time.
            RoomCreationStrategy::AuthenticatedApiRequest => options.room_key.clone().map(|key| {
                options
                    .app
                    .app_name_and_org_slug()
                    .with_room_key(RoomKey::Custom(key))
            }),
        };

        let notify_signal = if let Some(key_and_app) = check_pending_room.as_ref() {
            let mut pending_room_creations = self.0.pending_room_creations.lock().await;
            let pending_room = pending_room_creations.get(key_and_app);

            if let Some(ready_signal) = pending_room.cloned() {
                match options.creation_strategy {
                    RoomCreationStrategy::AutoCreate => {
                        // If the room is being created with the AutoCreate strategy, we can wait
                        // for the room to be created and then return it.
                        drop(pending_room_creations);

                        const WAIT_TIMEOUT: Duration = Duration::from_secs(10);
                        if tokio::time::timeout(WAIT_TIMEOUT, ready_signal.notified())
                            .await
                            // Timed out waiting for the room to be created.
                            .is_err()
                        {
                            // TODO: better error type, as we might want to return an error through
                            // the API
                            anyhow::bail!(
                                "timed out waiting for room with key {:?} to be created for app {}",
                                key_and_app.key,
                                key_and_app.app_id.app
                            );
                        }

                        return self
                            .get_by_key(options.app, key_and_app.key.clone())
                            .await
                            .context("failed to find room that was just created!");
                    }
                    RoomCreationStrategy::AuthenticatedApiRequest => {
                        // TODO: better error type here too
                        anyhow::bail!(
                            "room with key {:?} is already being created for app {}",
                            key_and_app.key,
                            key_and_app.app_id.app
                        );
                    }
                }
            } else {
                // Add the room to the pending rooms set so that other calls to create a room with
                // the same key will wait for this call to finish.
                let notify = Arc::new(Notify::new());
                pending_room_creations.insert(key_and_app.clone(), notify.clone());
                Some(notify)
            }
        } else {
            // If the room does not have any keys that need to be checked against pending rooms,
            // then we don't need to notify anyone when the room is created and we don't need to add
            // it to the pending rooms set.
            None
        };

        let result = self.do_create_room(options).await;

        if let Some(key_and_app) = check_pending_room {
            // TODO: refactor to type level guarantee instead of runtime error
            let signal =
                notify_signal.context("check_pending_room is Some but missing notify_signal!")?;

            // Remove the room from the pending rooms set and notify any waiters that the room has
            // been created.
            let mut pending_room_creations = self.0.pending_room_creations.lock().await;
            pending_room_creations.remove(&key_and_app);
            signal.notify_waiters();
            // TODO: handle errors during room creation and notify waiters that the room creation
            // failed
        }

        result
    }

    async fn do_create_room(&self, options: CreateRoomOptions<'_>) -> anyhow::Result<RoomCore<R>> {
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
        let host = self.host()?;
        let bundle = host.load_bundle_for_app(options.app).await?;

        let room_core = RoomCore::new(
            host,
            CreateRoomCoreOptions {
                bundle,
                meta: options.meta,
                app: options.app.app_name_and_org_slug(),
                // TODO: make this configurable based on the app's config
                resource_limit: ContainerResourceLimit::small_defaults(),
                extra_keys,
                creation_strategy: options.creation_strategy,
            },
        )
        .await?;

        let app_id = room_core.app().clone();

        {
            let mut maps = self.0.maps.write().await;

            // Update metadata about the room in the storage maps.
            maps.rooms.insert(room_core.id(), room_core.clone());

            maps.app_to_rooms
                .entry(app_id.clone())
                .or_default()
                .insert(room_core.id());

            // Some mappings depend on the room creation strategy, so we need to handle them
            // separately.
            match options.creation_strategy {
                RoomCreationStrategy::AutoCreate => {
                    maps.auto_created_rooms
                        .insert(app_id.clone(), room_core.id());
                }
                RoomCreationStrategy::AuthenticatedApiRequest => {
                    maps.api_created_rooms
                        .entry(app_id.clone())
                        .or_default()
                        .insert(room_core.id());
                }
            }

            // Add the room to the keys_to_rooms map for each of its keys.
            for key in room_core.keys() {
                maps.keys_to_rooms.insert(
                    RoomKeyAndApp {
                        app_id: app_id.clone(),
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
        let mut maps = self.0.maps.write().await;

        // Remove the room from the app_to_rooms map.
        let room = match maps.rooms.remove(room_id) {
            Some(room) => room,
            None => return None,
        };

        // Remove the room from the app_to_rooms map.
        maps.app_to_rooms
            .entry(room.app().clone())
            .or_default()
            .remove(room_id);

        // Remove the room from the auto_created_rooms map if it was auto-created.
        maps.auto_created_rooms.retain(|_, v| v != room_id);

        // Remove the room from the api_created_rooms map if it was created through the API.
        if let Some(api_created_room_ids) = maps.api_created_rooms.get_mut(room.app()) {
            api_created_room_ids.remove(room_id);
        }

        // Remove the room from the keys_to_rooms map.
        maps.keys_to_rooms.retain(|_, v| v != room_id);

        Some(room)
    }

    /// Gets the room that was automatically created for the given app, if it exists. Returns `None`
    /// if the app does not have an automatically created room or if the room was removed from
    /// storage.
    pub async fn get_autocreated_room(&self, app: &AppNameAndOrgSlug) -> Option<RoomCore<R>> {
        let maps = self.0.maps.read().await;

        maps.auto_created_rooms
            .get(app)
            .and_then(|room_id| maps.rooms.get(room_id).cloned())
    }
}
