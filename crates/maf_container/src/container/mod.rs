mod exports;
mod io;

use io::ContainerStdoutFactory;
use tokio::sync::mpsc;
use wasmtime as wt;
use wasmtime_wasi::IoView;

use crate::{api::connection::ConnectionHandle, runtime::wasi::Bindings};

pub struct Container {
    pub(super) path: String,
    pub(super) instance: Bindings,
    pub(super) store: wt::Store<ContainerData>,
    pub output: Option<mpsc::Receiver<String>>,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("path", &self.path)
            .field("store", &self.store)
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

pub struct ContainerData {
    pub resources: wasmtime_wasi::ResourceTable,
    pub wasi_ctx: wasmtime_wasi::WasiCtx,
    pub connection_tx: mpsc::Sender<ConnectionHandle>,
    pub connection_rx: Option<mpsc::Receiver<ConnectionHandle>>,
}

// TODO: make container data threadsafe in the future by ensuring that all accesses to wasm
// resources happen in the same thread. this can be done by ensuring that all communication with
// the wasm module is done via channels, with rx handled by a single thread
unsafe impl Sync for ContainerData {}

impl std::fmt::Debug for ContainerData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerData")
            .field("resources", &self.resources)
            .finish_non_exhaustive()
    }
}

impl Container {
    pub async fn load_from_file(
        runtime: &super::ContainerRuntime,
        path: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;

        let component = wt::component::Component::new(&runtime.engine, &bytes)?;

        let (connection_tx, connection_rx) = mpsc::channel(10);
        let (output_tx, output_rx) = mpsc::channel(100);
        let resources = wasmtime_wasi::ResourceTable::default();
        let stdout = ContainerStdoutFactory {
            output_tx: output_tx.clone(),
        };
        let wasi_ctx = wasmtime_wasi::WasiCtx::builder().stdout(stdout).build();
        let mut store = wt::Store::new(
            &runtime.engine,
            ContainerData {
                resources,
                wasi_ctx,
                connection_tx,
                connection_rx: Some(connection_rx),
            },
        );

        store.epoch_deadline_async_yield_and_update(1);

        let instance = Bindings::instantiate_async(&mut store, &component, &runtime.linker).await?;

        tracing::info!("loaded container `{}`", path);

        Ok(Self {
            path: path.to_string(),
            instance,
            store,
            output: Some(output_rx),
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        Ok(self
            .instance
            .call_run(&mut self.store)
            .await?
            .map_err(|_| anyhow::anyhow!("failed to init due to wasm exception"))?)
    }

    pub fn take_output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output.take()
    }
}

impl wasmtime_wasi::WasiView for ContainerData {
    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.wasi_ctx
    }
}

impl IoView for ContainerData {
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.resources
    }
}

fn a<T: Send + Sync + 'static>() {}
