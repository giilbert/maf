mod bridge;
pub mod wasi;

use crate::container::ContainerData;
use wasmtime as wt;

pub struct ContainerRuntime {
    pub(super) engine: wt::Engine,
    pub(super) linker: wt::component::Linker<ContainerData>,
}

impl ContainerRuntime {
    pub fn new() -> anyhow::Result<Self> {
        let engine = wt::Engine::new(
            &wt::Config::new()
                .wasm_memory64(false)
                .async_support(true)
                .epoch_interruption(true),
        )?;
        let linker = Self::create_component_linker(&engine)?;

        Ok(Self { engine, linker })
    }
}
