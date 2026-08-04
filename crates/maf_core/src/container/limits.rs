use std::sync::Arc;

use async_trait::async_trait;
use wasmtime::ResourceLimiterAsync;

use crate::ContainerResourceLimit;
use crate::container::{ContainerHandle, ContainerResourceStats};

// TODO: Pass in container data to the limiter
pub struct ContainerResourceLimiter {
    container: ContainerHandle,
    stats: Arc<ContainerResourceStats>,
    limits: ContainerResourceLimit,
}

impl ContainerResourceLimiter {
    pub fn new(
        container: ContainerHandle,
        stats: Arc<ContainerResourceStats>,
        limits: ContainerResourceLimit,
    ) -> Self {
        Self {
            container,
            stats,
            limits,
        }
    }
}

#[async_trait]
impl ResourceLimiterAsync for ContainerResourceLimiter {
    async fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        // TODO: use spans for logging
        tracing::debug!(
            "room {}: memory growing requested. current={}, desired={}, maximum={:?}",
            self.container.room_id(),
            current,
            desired,
            maximum
        );

        if desired > self.limits.memory {
            // TODO: use spans for logging
            tracing::warn!(
                "room {}: memory growing request denied. desired={} exceeds limit={}",
                self.container.room_id(),
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
    ) -> wasmtime::Result<bool> {
        // TODO: use spans for logging
        tracing::debug!(
            "room {}: table growing requested. current={}, desired={}, maximum={:?}",
            self.container.room_id(),
            current,
            desired,
            maximum
        );

        if desired > self.limits.table {
            // TODO: use spans for logging
            tracing::warn!(
                "room {}: table growing request denied. desired={} exceeds limit={}",
                self.container.room_id(),
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
