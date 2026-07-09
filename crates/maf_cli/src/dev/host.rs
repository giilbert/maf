use maf_core::server::{RoomHostImpl, RoomsStorage};

#[derive(Debug)]
pub struct DevServerRoomHost {
    storage: RoomsStorage,
    container_runtime: maf_core::ContainerRuntime,
}

impl RoomHostImpl for DevServerRoomHost {
    fn container_runtime(&self) -> &maf_core::ContainerRuntime {
        &self.container_runtime
    }

    fn room_storage(&self) -> &RoomsStorage {
        &self.storage
    }
}
