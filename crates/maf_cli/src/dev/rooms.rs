use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use maf_container::server::RoomInner;
use schemas::apps::{generate_room_secret, RoomCreationStrategy, RoomId};
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::config::ProjectConfig;

// Simplified version of RoomKeyHash and InsertRoom for development purposes

pub type RoomKey = String;

#[derive(Debug, Clone)]
pub struct InsertRoom {
    pub strategy: RoomCreationStrategy,
    pub room: RoomInner,
    pub key: String,
}

/// Rooms storage for development server.
///
/// Compared to `RoomsStorage` in `maf_container_host`, this is a simplified version that allows
/// more observability and less indexing for the development server.
#[derive(Debug, Clone)]
pub struct DevRoomsStorage {
    pub config: Option<ProjectConfig>,
    pub inner: Arc<RwLock<HashMap<RoomId, DevRoom>>>,
    pub keys: Arc<RwLock<HashMap<RoomKey, RoomId>>>,
    pub auto_created_room: Arc<RwLock<Option<RoomId>>>,
    pub api_created_rooms: Arc<RwLock<HashSet<RoomId>>>,
}

#[derive(Debug, Clone)]
pub struct DevRoom {
    pub id: RoomId,
    pub meta: DevRoomMeta,
    pub inner: RoomInner,
}

#[derive(Debug, Clone)]
pub struct DevRoomMeta {
    pub id: RoomId,
    pub secret: String,
    pub strategy: RoomCreationStrategy,
    pub key: String,
}

impl DevRoomsStorage {
    pub fn new(config: Option<ProjectConfig>) -> Self {
        Self {
            config,
            inner: Default::default(),
            keys: Default::default(),
            auto_created_room: Default::default(),
            api_created_rooms: Default::default(),
        }
    }

    pub async fn get(&self, room_id: &RoomId) -> Option<RwLockReadGuard<DevRoom>> {
        RwLockReadGuard::try_map(self.inner.read().await, |rooms| rooms.get(room_id)).ok()
    }

    pub async fn get_by_key_or_id(&self, key: &str) -> Option<RwLockReadGuard<DevRoom>> {
        // If the key is a UUID, we try to get the room by ID.
        if let Ok(uuid) = RoomId::parse_str(key) {
            return self.get(&uuid).await;
        }

        self.get(&self.keys.read().await.get(key).cloned()?).await
    }

    pub async fn insert(&self, param: InsertRoom) -> DevRoomMeta {
        let meta = DevRoomMeta {
            id: param.room.id(),
            secret: generate_room_secret(),
            strategy: param.strategy.clone(),
            key: param.key.clone(),
        };

        match param.strategy {
            RoomCreationStrategy::AutoCreate => {
                *self.auto_created_room.write().await = Some(param.room.id());
            }
            RoomCreationStrategy::AuthenticatedApiRequest => {
                self.api_created_rooms.write().await.insert(param.room.id());
            }
        }

        self.inner.write().await.insert(
            param.room.id(),
            DevRoom {
                id: param.room.id(),
                meta: meta.clone(),
                inner: param.room,
            },
        );

        meta
    }

    pub async fn remove(&self, room_id: &RoomId) -> Option<DevRoom> {
        let room = self.inner.write().await.remove(room_id)?;
        self.keys.write().await.remove(room.meta.key.as_str());

        *self.auto_created_room.write().await = None;
        self.api_created_rooms.write().await.remove(&room_id);

        Some(room)
    }
}
