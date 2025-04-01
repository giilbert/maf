use wasmtime as wt;

use super::{Container, ContainerData};

pub struct ContainerExports {
    pub(super) init: wt::TypedFunc<(), ()>,
    pub(super) alloc: wt::TypedFunc<(u32, u32), u32>,
    pub(super) dealloc: wt::TypedFunc<(u32, u32, u32), ()>,
}

impl std::fmt::Debug for ContainerExports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerExports").finish_non_exhaustive()
    }
}

impl ContainerExports {
    pub(super) fn new(
        instance: &wt::Instance,
        mut store: &mut wt::Store<ContainerData>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            init: instance.get_typed_func::<(), ()>(&mut store, "init")?,
            alloc: instance.get_typed_func::<(u32, u32), u32>(&mut store, "alloc")?,
            dealloc: instance.get_typed_func::<(u32, u32, u32), ()>(&mut store, "dealloc")?,
        })
    }
}
