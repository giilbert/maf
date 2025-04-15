mod exports;
mod io;

use std::{
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use io::ContainerStdoutFactory;
use tokio::{
    sync::{mpsc, oneshot},
    time,
};
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
    pub(crate) last_activity: Arc<AtomicU64>,
}

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
                last_activity: Arc::new(AtomicU64::new(now_as_secs())),
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
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        // Spawn a task to monitor inactivity and stop the container
        let last_activity = self.store.data().last_activity.clone();
        tokio::spawn(async move {
            loop {
                const CHECK_INTERVAL: u64 = 5; // seconds
                const TIMEOUT: u64 = 60; // seconds

                time::sleep(Duration::from_secs(CHECK_INTERVAL)).await;

                // Check if the container has been inactive for more than TIMEOUT seconds
                if now_as_secs() - last_activity.load(Ordering::Relaxed) > TIMEOUT {
                    tracing::info!("container is inactive for too long, stopping...");
                    let _ = stop_tx.send(());
                    break;
                }
            }
        });

        tokio::select! {
            result = self.instance.call_run(&mut self.store) => {
                let inner_result = result?;
                return inner_result.map_err(|e| anyhow::anyhow!("container error: {e:?}"));
            }
            _ = stop_rx => {
                tracing::info!("container stopped due to inactivity");
            }
        }

        Ok(())
    }

    pub fn take_output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output.take()
    }
}

impl ContainerData {
    pub fn update_last_activity(&self) {
        self.last_activity.store(now_as_secs(), Ordering::Relaxed);
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

fn now_as_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs()
}
