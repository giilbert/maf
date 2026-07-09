use std::collections::{HashMap, HashSet};

use maf_schemas::apps::{AppNameAndOrgSlug, RoomCreationStrategy, RoomId, RoomKeyHash};
use tokio::sync::RwLock;

use crate::server::RoomCore;
use crate::server::room::RoomHost;

/// A struct representing all rooms on the server, allowing for lookup and management of rooms.
///
/// Normally, this would be accessed through [`RoomHost`] or [`RoomHostImpl`].
#[derive(Debug, Default)]
pub struct RoomsStorage {
    /// All rooms on the server. Mapped by their unique [`RoomId`].
    rooms: RwLock<HashMap<RoomId, RoomCore>>,

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

impl RoomsStorage {
    /// Gets a room by its ID. Returns `None` if no room with the given ID exists.
    pub async fn get(&self, room_id: &RoomId) -> Option<RoomCore> {
        let rooms = self.rooms.read().await;
        rooms.get(room_id).cloned()
    }

    /// Removes a room by its ID. This is used when a room is shut down and should not accept any
    /// newer connections or be returned in lookups.
    ///
    /// FIXME: is there a race condition with autocreated rooms here?
    pub async fn remove(&self, room_id: &RoomId) -> Option<RoomCore> {
        let mut rooms = self.rooms.write().await;

        let mut auto_created_rooms = self.auto_created_rooms.write().await;
        let mut api_created_rooms = self.api_created_rooms.write().await;
        let mut keys_to_rooms = self.keys_to_rooms.write().await;

        rooms.remove(room_id);

        todo!();
    }

    pub async fn get_autocreated_room(&self, app: &AppNameAndOrgSlug) -> Option<RoomCore> {
        let rooms = self.rooms.read().await;
        let auto_created_rooms = self.auto_created_rooms.read().await;

        auto_created_rooms
            .get(app)
            .and_then(|room_id| rooms.get(room_id).cloned())
    }
}
