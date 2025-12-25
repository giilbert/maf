use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use dashmap::{DashMap, DashSet};
use maf_container::{server::RoomInner, ContainerResourceLimit};
use maf_schemas::{
    apps::{generate_room_secret, AppNameAndOrgSlug, RoomCreationStrategy, RoomId, RoomKeyHash},
    error::ErrorResponse,
};
use tokio::sync::{Notify, RwLock, RwLockReadGuard};

use crate::{
    api::{AppState, Environment},
    storage::db::app,
};

#[derive(Debug, Clone)]
pub struct Room {
    pub id: RoomId,
    pub meta: RoomMeta,
    pub inner: RoomInner,
}

/// Contains additional information about the room, not related to the running the container.
#[derive(Debug, Clone)]
pub struct RoomMeta {
    pub id: RoomId,
    pub app: AppNameAndOrgSlug,
    /// This is used to create and verify JWT payloads.
    pub secret: String,
    /// The room creation strategy used to create this room. Needed to determine how to handle room
    /// removal and access.
    pub strategy: RoomCreationStrategy,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct InsertRoom {
    pub strategy: RoomCreationStrategy,
    pub app: AppNameAndOrgSlug,
    pub room: RoomInner,
    pub key: String,
}

#[derive(Debug, Clone, Default)]
pub struct RoomsStorage {
    pub inner: Arc<RwLock<HashMap<RoomId, Room>>>,
    pub keys: Arc<RwLock<HashMap<RoomKeyHash, RoomId>>>,
    pub auto_created_rooms: Arc<RwLock<HashMap<AppNameAndOrgSlug, RoomId>>>,
    pub api_created_rooms: Arc<RwLock<HashMap<AppNameAndOrgSlug, HashSet<RoomId>>>>,
    /// A set of autocreate rooms that are currently being created to prevent race conditions.
    creating_autocreate_rooms: Arc<DashSet<AppNameAndOrgSlug>>,
    autocreate_room_notify: Arc<DashMap<AppNameAndOrgSlug, Arc<Notify>>>,
}

impl RoomsStorage {
    pub async fn get(&self, room_id: &RoomId) -> Option<RwLockReadGuard<'_, Room>> {
        RwLockReadGuard::try_map(self.inner.read().await, |rooms| rooms.get(room_id)).ok()
    }

    pub async fn get_by_key_or_id(
        &self,
        app: &AppNameAndOrgSlug,
        key: &str,
    ) -> Option<RwLockReadGuard<'_, Room>> {
        // If the key is a UUID, we try to get the room by ID.
        if let Ok(uuid) = RoomId::parse_str(key) {
            return self.get(&uuid).await;
        }

        // Otherwise, we try to get the room by key.
        let keys = self.keys.read().await;
        let key = keys
            .get(&RoomKeyHash {
                app: app.clone(),
                key: key.to_string(),
            })
            .cloned()?;

        RwLockReadGuard::try_map(self.inner.read().await, |rooms| rooms.get(&key)).ok()
    }

    /// Insert a room into the storage with a given strategy and metadata.
    pub async fn insert(&self, param: InsertRoom) -> RoomMeta {
        let meta = RoomMeta {
            id: param.room.id(),
            app: param.app.clone(),
            secret: generate_room_secret(),
            strategy: param.strategy.clone(),
            key: param.key.clone(),
        };

        match param.strategy {
            RoomCreationStrategy::AutoCreate => {
                self.auto_created_rooms
                    .write()
                    .await
                    .insert(param.app.clone(), param.room.id());
            }
            RoomCreationStrategy::AuthenticatedApiRequest => {
                self.api_created_rooms
                    .write()
                    .await
                    .entry(param.app.clone())
                    .or_default()
                    .insert(param.room.id());
            }
        }

        // Insert keys into keys index
        let mut keys = self.keys.write().await;
        keys.insert(
            RoomKeyHash {
                app: param.app.clone(),
                key: param.key.clone(),
            },
            param.room.id(),
        );

        self.inner.write().await.insert(
            param.room.id(),
            Room {
                id: param.room.id(),
                meta: meta.clone(),
                inner: param.room,
            },
        );

        meta
    }

    /// Removes a room from the storage and returns it if it exists.
    /// The room is automatically removed from the auto-created or API-created rooms list based on
    /// the strategy used to create it.
    pub async fn remove(&self, room_id: &RoomId) -> Option<Room> {
        let room = self.inner.write().await.remove(room_id)?;

        match room.meta.strategy {
            RoomCreationStrategy::AutoCreate => {
                self.auto_created_rooms.write().await.remove(&room.meta.app);
            }
            RoomCreationStrategy::AuthenticatedApiRequest => {
                self.api_created_rooms
                    .write()
                    .await
                    .entry(room.meta.app.clone())
                    .and_modify(|rooms| {
                        rooms.remove(room_id);
                    });
            }
        }

        // Remove keys from keys index
        let mut keys = self.keys.write().await;
        keys.remove(&RoomKeyHash {
            app: room.meta.app.clone(),
            key: room.meta.key.clone(),
        });

        Some(room)
    }

    /// Finds the autocreated room for the given app and org slug, creating it if it does not exist.
    #[tracing::instrument(skip(self, state))]
    pub async fn fetch_autocreated_room(
        &self,
        state: &AppState,
        app: Option<app::Model>,
        org_slug: String,
    ) -> Result<Room, ErrorResponse> {
        let app_org = AppNameAndOrgSlug {
            app: app
                // In development, if the app is not found, we use the test app
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or("development".to_string()),
            org: org_slug.clone(),
        };

        // The code is structured this way to avoid deadlocks
        let existing_room_id = self.auto_created_rooms.read().await.get(&app_org).cloned();

        let room_id = match existing_room_id {
            Some(id) => id,
            // Room does not exist, create it
            None => {
                // Check if another task is already creating this room. If so, wait for it to finish
                // and then return the room.

                // If entry is newly inserted, this task is responsible for creating the room.
                let is_first = self.creating_autocreate_rooms.insert(app_org.clone());

                if !is_first {
                    // Another task is creating the room, wait for it to finish.
                    let notify = self
                        .autocreate_room_notify
                        .entry(app_org.clone())
                        .or_insert_with(|| Arc::new(Notify::new()))
                        .clone();

                    // Double-check in case the room is already available to avoid a missed-notify
                    // race (Notify does not buffer past signals).
                    if let Some(room) = self.get_by_key_or_id(&app_org, "default").await {
                        return Ok(room.clone());
                    }

                    notify.notified().await;

                    return Ok(self
                        .get_by_key_or_id(&app_org, "default")
                        .await
                        .ok_or_else(|| anyhow::anyhow!("Room not found after creation"))?
                        .clone());
                }

                // This task is responsible for creating the room.
                self.autocreate_room_notify
                    .entry(app_org.clone())
                    .or_insert_with(|| Arc::new(Notify::new()));

                // Re-check if the room was created by another task between the initial read and
                // acquiring "leadership" (is_first=true).
                if let Some(already_id) =
                    self.auto_created_rooms.read().await.get(&app_org).cloned()
                {
                    if let Some(n) = self.autocreate_room_notify.get(&app_org) {
                        n.notify_waiters();
                    }
                    self.autocreate_room_notify.remove(&app_org);
                    self.creating_autocreate_rooms.remove(&app_org);

                    return Ok(state
                        .rooms
                        .get(&already_id)
                        .await
                        .expect("room not found")
                        .clone());
                }

                // Perform creation inside an inner async block so we can always run cleanup
                // (notify + set removals) even on early errors.
                let result: Result<RoomId, ErrorResponse> = async {
                    let (room, mut container) = match &app {
                        Some(app) => {
                            RoomInner::new(
                                &state.container_runtime,
                                state
                                    .bundle_storage
                                    .load_app_bundle(app.id)
                                    .await?
                                    .ok_or_else(|| {
                                        ErrorResponse::not_found(Some("app bundle not found"))
                                    })?,
                                ContainerResourceLimit::sensible_default(),
                            )
                            .await?
                        }
                        None if state.environment == Environment::Development => {
                            tracing::info!(
                                "App not found. Defaulting to test app (development only)"
                            );
                            RoomInner::new(
                                &state.container_runtime,
                                state.bundle_storage.load_test_app().await?,
                                ContainerResourceLimit::sensible_default(),
                            )
                            .await?
                        }
                        None => return Err(ErrorResponse::not_found(Some("app not found"))),
                    };

                    let room_id = room.id();
                    state
                        .rooms
                        .insert(InsertRoom {
                            room,
                            strategy: RoomCreationStrategy::AutoCreate,
                            app: app_org.clone(),
                            key: "default".to_string(),
                        })
                        .await;

                    let state = state.clone();
                    container.pass_output();
                    container.start_inactive_shutdown_task();

                    tokio::spawn(async move {
                        if let Err(e) = container.run().await {
                            tracing::error!("container {} error: {e:?}", container.room_id);
                        }
                        tracing::info!("container {} stopped", container.room_id);

                        state.rooms.remove(&room_id).await;
                    });

                    Ok(room_id)
                }
                .await;

                // Notify other tasks waiting for this room to be created and clean up, regardless
                // of success or error.
                if let Some(n) = self.autocreate_room_notify.get(&app_org) {
                    n.notify_waiters();
                }
                self.autocreate_room_notify.remove(&app_org);
                self.creating_autocreate_rooms.remove(&app_org);

                // Return based on creation result
                match result {
                    Ok(room_id) => room_id,
                    Err(e) => return Err(e),
                }
            }
        };

        Ok(state
            .rooms
            .get(&room_id)
            .await
            .expect("room not found")
            .clone())
    }
}
