use std::sync::Arc;

use async_trait::async_trait;
use cobble_schemas::apps::RoomId;
use wasmtime::ResourceLimiterAsync;

use crate::{ContainerResourceLimit, container::ContainerResourceStats};

// TODO: Pass in container data to the limiter
pub struct ContainerResourceLimiter {
    pub(crate) room_id: RoomId,
    pub(crate) stats: Arc<ContainerResourceStats>,
    pub(crate) limits: ContainerResourceLimit,
}

#[async_trait]
impl ResourceLimiterAsync for ContainerResourceLimiter {
    async fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        tracing::debug!(
            "Room {}: memory growing requested. current={}, desired={}, maximum={:?}",
            self.room_id,
            current,
            desired,
            maximum
        );

        if desired > self.limits.memory {
            tracing::warn!(
                "Room {}: memory growing request denied. desired={} exceeds limit={}",
                self.room_id,
                desired,
                self.limits.memory
            );

            return Ok(false);
        }

        self.stats
            .memory_usage
            .store(desired, std::sync::atomic::Ordering::Relaxed);

        Ok(true)
    }

    async fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        tracing::debug!(
            "Room {}: table growing requested. current={}, desired={}, maximum={:?}",
            self.room_id,
            current,
            desired,
            maximum
        );

        if desired > self.limits.table {
            tracing::warn!(
                "Room {}: table growing request denied. desired={} exceeds limit={}",
                self.room_id,
                desired,
                self.limits.table
            );

            return Ok(false);
        }

        self.stats
            .table_usage
            .store(desired, std::sync::atomic::Ordering::Relaxed);

        Ok(true)
    }
}
