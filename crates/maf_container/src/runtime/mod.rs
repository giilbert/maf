mod bridge;

use crate::container::ContainerData;
use wasmtime as wt;

pub struct ContainerRuntime {
    pub(super) engine: wt::Engine,
    pub(super) linker: wt::Linker<ContainerData>,
}

impl ContainerRuntime {
    pub fn new() -> anyhow::Result<Self> {
        let engine = wt::Engine::new(
            &wt::Config::new()
                .wasm_memory64(false)
                .async_support(true)
                .epoch_interruption(true),
        )?;
        let linker = Self::create_linker_with_ffi(&engine)?;

        Ok(Self { engine, linker })
    }
}
