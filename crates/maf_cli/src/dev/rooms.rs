use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::Context;
use colored::Colorize as _;
use maf_container::{
    server::{Bundle, CreateRoomInnerOptions, RoomInner},
    Container, ContainerResourceLimit, ContainerRuntime,
};
use maf_schemas::apps::{generate_room_secret, MetaEntryMap, RoomCreationStrategy, RoomId};
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::{config::ProjectConfig, dev::dev_server::DevServerState};

// Simplified version of RoomKeyHash and InsertRoom for development purposes

pub type RoomKey = String;

#[derive(Debug, Clone)]
pub struct InsertRoom {
    pub strategy: RoomCreationStrategy,
    pub key: Option<String>,
    pub meta: Option<MetaEntryMap>,
}

/// Rooms storage for development server.
///
/// Compared to `RoomsStorage` in `maf_container_host`, this is a simplified version that allows
/// more observability and less indexing for the development server.
#[derive(Debug)]
pub struct DevRoomsStorage {
    pub bundle: Bundle,
    pub _config: Option<ProjectConfig>,
    pub inner: RwLock<HashMap<RoomId, DevRoom>>,
    pub keys: RwLock<HashMap<RoomKey, RoomId>>,
    pub auto_created_room: RwLock<Option<RoomId>>,
    pub api_created_rooms: RwLock<HashSet<RoomId>>,
}

#[derive(Debug, Clone)]
pub struct DevRoom {
    pub id: RoomId,
    pub meta: DevRoomMeta,
    pub inner: RoomInner,
}

impl DevRoom {
    pub async fn new(
        runtime: &ContainerRuntime,
        bundle: &Bundle,
        meta: Option<MetaEntryMap>,
    ) -> anyhow::Result<(Self, Container)> {
        let (inner, container) = RoomInner::new(
            runtime,
            CreateRoomInnerOptions {
                bundle: bundle.clone(),
                resource_limit: ContainerResourceLimit {
                    memory: 256 * 1024 * 1024, // 256 MB
                    table: usize::MAX,
                },
                meta,
            },
        )
        .await
        .context("failed to create room container")?;

        Ok((
            Self {
                id: inner.id(),
                meta: DevRoomMeta {
                    id: inner.id(),
                    secret: generate_room_secret(),
                    _strategy: RoomCreationStrategy::AutoCreate,
                    key: inner.id().to_string(),
                },
                inner,
            },
            container,
        ))
    }

    pub fn id(&self) -> RoomId {
        self.id.clone()
    }
}

#[derive(Debug, Clone)]
pub struct DevRoomMeta {
    pub id: RoomId,
    pub secret: String,
    pub _strategy: RoomCreationStrategy,
    pub key: String,
}

impl DevRoomsStorage {
    pub fn new(
        wasm_module_path: impl AsRef<Path>,
        config: Option<ProjectConfig>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            _config: config,
            bundle: Bundle::load_wasm_module_from_file(&wasm_module_path).with_context(|| {
                format!(
                    "failed to load WASM module from `{}`",
                    wasm_module_path.as_ref().display()
                )
            })?,
            inner: Default::default(),
            keys: Default::default(),
            auto_created_room: Default::default(),
            api_created_rooms: Default::default(),
        })
    }

    pub async fn get(&self, room_id: &RoomId) -> Option<RwLockReadGuard<'_, DevRoom>> {
        RwLockReadGuard::try_map(self.inner.read().await, |rooms| rooms.get(room_id)).ok()
    }

    pub async fn get_by_key_or_id(&self, key: &str) -> Option<RwLockReadGuard<'_, DevRoom>> {
        // If the key is a UUID, we try to get the room by ID.
        if let Ok(uuid) = RoomId::parse_str(key) {
            return self.get(&uuid).await;
        }

        self.get(&self.keys.read().await.get(key).cloned()?).await
    }

    pub async fn insert(
        &self,
        state: &DevServerState,
        param: InsertRoom,
    ) -> anyhow::Result<DevRoom> {
        let (room, mut container) = DevRoom::new(&state.runtime, &self.bundle, param.meta).await?;
        let room_key = param.key.unwrap_or(room.id().to_string());
        let output_finished_token = tokio_util::sync::CancellationToken::new();

        let room_key_clone = room_key.clone();
        let output_finished_token_clone = output_finished_token.clone();
        let mut output = container.output().expect("Output should be available");
        tokio::spawn(async move {
            while let Some(line) = output.recv().await {
                let line = line.trim_matches(&['\n', '\r', ' ']);
                println!(
                    "{} {} {line}",
                    format!("[dev] `{}`", room_key_clone).dimmed(),
                    "out".blue()
                )
            }

            print_dimmed!("[dev] `{}` output finished", room_key_clone);
            output_finished_token_clone.cancel();
        });

        let room_key_clone = room_key.clone();
        tokio::spawn(async move {
            if let Err(e) = container.run().await {
                drop(container);
                output_finished_token.cancelled().await;
                println!(
                    "{}",
                    format!(
                        "[dev] `{}` Error running room container: {e}",
                        room_key_clone
                    )
                    .red()
                )
            }
        });

        let meta = DevRoomMeta {
            id: room.id(),
            secret: generate_room_secret(),
            _strategy: param.strategy.clone(),
            key: room_key.clone(),
        };

        match param.strategy {
            RoomCreationStrategy::AutoCreate => {
                *self.auto_created_room.write().await = Some(room.id());
            }
            RoomCreationStrategy::AuthenticatedApiRequest => {
                self.api_created_rooms.write().await.insert(room.id());
            }
        }

        self.inner.write().await.insert(room.id(), room.clone());

        self.keys
            .write()
            .await
            .insert(room_key.clone(), meta.id.clone());

        println!("[dev] Created room with key `{}`", meta.key);

        Ok(room)
    }

    #[allow(dead_code)]
    pub async fn remove(&self, room_id: &RoomId) -> Option<DevRoom> {
        let room = self.inner.write().await.remove(room_id)?;
        self.keys.write().await.remove(room.meta.key.as_str());

        *self.auto_created_room.write().await = None;
        self.api_created_rooms.write().await.remove(&room_id);

        Some(room)
    }
}
