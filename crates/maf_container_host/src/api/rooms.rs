use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use maf_container::server::{Room, RoomId};
use rand::Rng as _;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppNameAndOrgSlug {
    pub app: String,
    pub org: String,
}

/// Contains additional information about the room, not related to the running the container.
#[derive(Debug, Clone)]
pub struct RoomMeta {
    pub app: AppNameAndOrgSlug,
    /// Optional secret for the room, as an extra layer of authentication.
    pub room_secret: String,
}

fn generate_room_secret() -> String {
    let mut rng = rand::rng();

    (0..64)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct RoomsStorage {
    rooms: Arc<RwLock<HashMap<RoomId, (RoomMeta, Room)>>>,
    pub auto_created_rooms: Arc<RwLock<HashMap<AppNameAndOrgSlug, RoomId>>>,
    pub api_created_rooms: Arc<RwLock<HashMap<AppNameAndOrgSlug, HashSet<RoomId>>>>,
}

impl RoomsStorage {
    pub async fn get(&self, room_id: &RoomId) -> Option<RwLockReadGuard<(RoomMeta, Room)>> {
        RwLockReadGuard::try_map(self.rooms.read().await, |rooms| rooms.get(room_id)).ok()
    }

    pub async fn get_auto_created_room(
        &self,
        app: &AppNameAndOrgSlug,
    ) -> Option<RwLockReadGuard<(RoomMeta, Room)>> {
        let auto_created_rooms = self.auto_created_rooms.read().await;
        let room_id = auto_created_rooms.get(app)?;
        self.get(room_id).await
    }

    pub async fn insert_auto_created_room(&self, room: Room, app: AppNameAndOrgSlug) -> RoomMeta {
        let meta = RoomMeta {
            app: app.clone(),
            room_secret: generate_room_secret(),
        };

        self.auto_created_rooms.write().await.insert(app, room.id);
        self.rooms
            .write()
            .await
            .insert(room.id, (meta.clone(), room));

        meta
    }

    pub async fn remove_auto_created_room(&self, app: &AppNameAndOrgSlug) {
        if let Some(room_id) = self.auto_created_rooms.write().await.remove(app) {
            self.rooms.write().await.remove(&room_id);
        }
    }

    pub async fn insert_api_room(&self, room: Room, app: AppNameAndOrgSlug) -> RoomMeta {
        let meta = RoomMeta {
            app: app.clone(),
            room_secret: generate_room_secret(),
        };

        self.rooms
            .write()
            .await
            .insert(room.id, (meta.clone(), room.clone()));

        self.api_created_rooms
            .write()
            .await
            .entry(app)
            .or_default()
            .insert(room.id);

        meta
    }

    pub async fn remove_api_room(&self, room_id: &RoomId, app: AppNameAndOrgSlug) {
        self.rooms.write().await.remove(room_id);
        self.api_created_rooms
            .write()
            .await
            .entry(app.clone())
            .and_modify(|rooms| {
                rooms.remove(&room_id);
            });

        // Remove the entry in api_created_rooms_by_org_slug if it's empty
        if self
            .api_created_rooms
            .read()
            .await
            .get(&app)
            .map_or(false, |rooms| rooms.is_empty())
        {
            self.api_created_rooms.write().await.remove(&app);
        }
    }
}
