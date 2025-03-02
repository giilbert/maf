mod abi;
mod exports;

use exports::ContainerExports;
use tokio::sync::mpsc;
use wasmtime as wt;

#[derive(Debug)]
pub struct Container {
    pub(super) path: String,
    pub(super) module: wt::Module,
    pub(super) instance: wt::Instance,
    pub(super) store: wt::Store<ContainerData>,
    pub(super) exports: ContainerExports,

    pub output: Option<mpsc::Receiver<String>>,
}

#[derive(Debug)]
pub struct ContainerData {
    // TODO: make a data structure that will will dequeue old messages
    pub output_tx: mpsc::Sender<String>,
}

impl Container {
    pub async fn load_from_file(
        runtime: &super::ContainerRuntime,
        path: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;

        let module = wt::Module::new(&runtime.engine, &bytes)?;

        let (output_tx, output_rx) = mpsc::channel(100);
        let mut store = wt::Store::new(&runtime.engine, ContainerData { output_tx });

        store.epoch_deadline_async_yield_and_update(1);

        let instance = runtime
            .linker
            .instantiate_async(&mut store, &module)
            .await?;

        let exports = ContainerExports::new(&instance, &mut store)?;

        println!("loaded container `{}`", path);

        Ok(Self {
            path: path.to_string(),
            module,
            instance,
            store,
            exports,
            output: Some(output_rx),
        })
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
        self.exports.init.call_async(&mut self.store, ()).await
    }

    pub fn take_output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output.take()
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
