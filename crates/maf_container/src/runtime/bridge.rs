use std::sync::atomic::AtomicI32;

use crate::container::ContainerData;
use wasmtime::{self as wt};

use super::{wasi::Bindings as CustomBindings, ContainerRuntime};

static NUMBER: AtomicI32 = AtomicI32::new(0);

impl ContainerRuntime {
    pub(super) fn create_component_linker(
        engine: &wt::Engine,
    ) -> anyhow::Result<wt::component::Linker<ContainerData>> {
        let mut linker = wt::component::Linker::new(engine);

        // CustomBindings::add_to_linker(&mut linker, |state: &mut ContainerData| state)?;
        wasmtime_wasi::add_to_linker_async(&mut linker)?;

        Ok(linker)
    }
}
