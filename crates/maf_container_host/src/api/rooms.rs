use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use maf_container::server::{RoomId, RoomInner};
use rand::Rng as _;
use schemas::apps::RoomCreationStrategy;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppNameAndOrgSlug {
    pub app: String,
    pub org: String,
}

/// Contains additional information about the room, not related to the running the container.
#[derive(Debug, Clone)]
pub struct RoomMeta {
    pub id: RoomId,
    pub app: AppNameAndOrgSlug,
    /// Optional secret for the room, as an extra layer of authentication.
    pub secret: String,
    /// The room creation strategy used to create this room. Needed to determine how to handle room
    /// removal and access.
    pub strategy: RoomCreationStrategy,
}

#[derive(Debug, Clone)]
pub struct Room {
    pub id: RoomId,
    pub meta: RoomMeta,
    pub inner: RoomInner,
}

fn generate_room_secret() -> String {
    let mut rng = rand::rng();

    (0..64)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect()
}

#[derive(Debug, Clone)]
pub struct InsertRoom {
    pub strategy: RoomCreationStrategy,
    pub app: AppNameAndOrgSlug,
    pub room: RoomInner,
}

#[derive(Debug, Clone, Default)]
pub struct RoomsStorage {
    rooms: Arc<RwLock<HashMap<RoomId, Room>>>,
    pub auto_created_rooms: Arc<RwLock<HashMap<AppNameAndOrgSlug, RoomId>>>,
    pub api_created_rooms: Arc<RwLock<HashMap<AppNameAndOrgSlug, HashSet<RoomId>>>>,
}

impl RoomsStorage {
    pub async fn get(&self, room_id: &RoomId) -> Option<RwLockReadGuard<Room>> {
        RwLockReadGuard::try_map(self.rooms.read().await, |rooms| rooms.get(room_id)).ok()
    }

    /// Insert a room into the storage with a given strategy and metadata.
    pub async fn insert(&self, param: InsertRoom) -> RoomMeta {
        let meta = RoomMeta {
            id: param.room.id(),
            app: param.app.clone(),
            secret: generate_room_secret(),
            strategy: param.strategy.clone(),
        };

        match param.strategy {
            RoomCreationStrategy::AutoCreate => {
                self.auto_created_rooms
                    .write()
                    .await
                    .insert(param.app, param.room.id());
            }
            RoomCreationStrategy::AuthenticatedApiRequest => {
                self.api_created_rooms
                    .write()
                    .await
                    .entry(param.app)
                    .or_default()
                    .insert(param.room.id());
            }
        }

        self.rooms.write().await.insert(
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
        let room = self.rooms.write().await.remove(room_id)?;

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

        Some(room)
    }
}
