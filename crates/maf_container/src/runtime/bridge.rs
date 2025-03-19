use std::sync::atomic::AtomicI32;

use crate::container::{Container, ContainerData};
use wasmtime::{self as wt};
use wasmtime_wasi::{IoImpl, IoView, WasiImpl, WasiView};

use super::{wasi, ContainerRuntime};

static NUMBER: AtomicI32 = AtomicI32::new(0);

impl ContainerRuntime {
    pub(super) fn create_component_linker(
        engine: &wt::Engine,
    ) -> anyhow::Result<wt::component::Linker<ContainerData>> {
        let mut linker = wt::component::Linker::new(engine);

        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        wasi::add_to_linker(&mut linker, |state| state)?;

        Ok(linker)
    }
}
