use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::runtime::ContainerRuntime;

use super::room::Room;

#[derive(Debug, Clone)]
pub struct AppState {
    pub container_runtime: ContainerRuntime,
    pub auto_created_rooms_by_org_slug: Arc<DashMap<String, Uuid>>,
    pub rooms: Arc<DashMap<Uuid, Room>>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let container_runtime = ContainerRuntime::init()?;

        Ok(Self {
            container_runtime,
            auto_created_rooms_by_org_slug: Arc::new(DashMap::new()),
            rooms: Arc::new(DashMap::new()),
        })
    }
}
