mod bridge;
pub mod wasi;

use std::{
    fmt::Debug,
    sync::{Arc, atomic::AtomicU64},
};

use crate::container::ContainerData;
use wasmtime as wt;

#[derive(Clone)]
pub struct ContainerRuntime {
    pub(super) engine: wt::Engine,
    pub(super) linker: wt::component::Linker<ContainerData>,
    pub(super) app_activity: &'static AtomicU64,
}

impl Debug for ContainerRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerRuntime").finish_non_exhaustive()
    }
}

impl ContainerRuntime {
    pub fn init(app_activity: &'static AtomicU64) -> anyhow::Result<Self> {
        let engine = wt::Engine::new(
            &wt::Config::new()
                .wasm_memory64(false)
                .async_support(true)
                .epoch_interruption(true),
        )?;
        let linker = Self::create_component_linker(&engine)?;

        Ok(Self {
            engine: engine.clone(),
            linker: linker.clone(),
            app_activity,
        })
    }
}
