pub mod wasi;

use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use wasmtime::component::HasSelf;
use wasmtime::{self as wt};

use crate::container::ContainerData;

#[derive(Clone)]
pub struct ContainerRuntime {
    pub(super) engine: wt::Engine,
    pub(super) linker: Arc<wt::component::Linker<ContainerData>>,
    pub(super) app_activity: &'static AtomicU64,
}

impl Debug for ContainerRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerRuntime").finish_non_exhaustive()
    }
}

impl ContainerRuntime {
    pub fn init(app_activity: &'static AtomicU64) -> anyhow::Result<Self> {
        let engine = wt::Engine::new(wt::Config::new().epoch_interruption(true))?;
        let linker = Self::create_component_linker(&engine)?;

        Ok(Self {
            engine,
            linker: Arc::new(linker),
            app_activity,
        })
    }

    pub(super) fn create_component_linker(
        engine: &wt::Engine,
    ) -> anyhow::Result<wt::component::Linker<ContainerData>> {
        let mut linker = wt::component::Linker::new(engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
        // // TODO: Limit HTTP bandwidth?
        wasi::bindings::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

        Ok(linker)
    }
}
