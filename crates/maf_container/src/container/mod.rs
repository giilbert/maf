mod abi;
mod exports;

use std::{collections::VecDeque, sync::mpsc};

use exports::ContainerExports;
use wasmtime as wt;

#[derive(Debug)]
pub struct Container {
    pub(super) path: String,
    pub(super) module: wt::Module,
    pub(super) instance: wt::Instance,
    pub(super) store: wt::Store<ContainerData>,
    pub(super) exports: ContainerExports,

    pub output: mpsc::Receiver<String>,
}

#[derive(Debug)]
pub struct ContainerData {
    // TODO: make a data structure that will will dequeue old messages
    pub(crate) output_tx: mpsc::SyncSender<String>,
}

impl Container {
    pub fn load_from_file(
        runtime: &super::ContainerRuntime,
        path: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;

        let module = wt::Module::new(&runtime.engine, &bytes)?;

        let (output_tx, output_rx) = mpsc::sync_channel(100);
        let mut store = wt::Store::new(&runtime.engine, ContainerData { output_tx });
        let instance = runtime.linker.instantiate(&mut store, &module)?;

        let exports = ContainerExports::new(&instance, &mut store)?;

        exports.init.call(&mut store, ())?;

        println!("loaded container `{}`", path);

        Ok(Self {
            path: path.to_string(),
            module,
            instance,
            store,
            exports,
            output: output_rx,
        })
    }

    pub fn get_memory(&mut self) -> anyhow::Result<wt::Memory> {
        self.instance
            .get_memory(&mut self.store, "memory")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "failed to find `memory` export in instance of `{}`",
                    self.path
                )
            })
    }
}
