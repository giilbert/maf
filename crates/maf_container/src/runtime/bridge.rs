use crate::container::ContainerData;
use wasmtime::{self as wt, component::HasSelf};

use super::{ContainerRuntime, wasi};

// static NUMBER: AtomicI32 = AtomicI32::new(0);

impl ContainerRuntime {
    pub(super) fn create_component_linker(
        engine: &wt::Engine,
    ) -> anyhow::Result<wt::component::Linker<ContainerData>> {
        let mut linker = wt::component::Linker::new(engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasi::bindings::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

        Ok(linker)
    }
}
