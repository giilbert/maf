mod io;

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use io::ContainerStdoutFactory;
use tokio::{sync::mpsc, time};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wasmtime as wt;
use wasmtime_wasi::IoView;

use crate::{interface::BoxedConnection, runtime::wasi::Bindings, utils, wasi::HookRequest};

pub struct Container {
    pub id: Uuid,
    pub instance: Bindings,
    pub store: wt::Store<ContainerData>,
    pub output: Option<mpsc::Receiver<String>>,
    pub cancel_token: CancellationToken,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("store", &self.store)
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

pub struct ContainerData {
    pub resources: wasmtime_wasi::ResourceTable,
    pub wasi_ctx: wasmtime_wasi::WasiCtx,
    pub connection_tx: mpsc::Sender<BoxedConnection>,
    pub hook_request_tx: mpsc::Sender<HookRequest>,
    pub connection_rx: Option<mpsc::Receiver<BoxedConnection>>,
    pub hook_request_rx: Option<mpsc::Receiver<HookRequest>>,
    pub(crate) last_activity: Arc<AtomicU64>,
    pub(crate) app_activity: &'static AtomicU64,
}

impl std::fmt::Debug for ContainerData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerData")
            .field("resources", &self.resources)
            .finish_non_exhaustive()
    }
}

impl Container {
    pub async fn load_from_binary(
        runtime: &super::ContainerRuntime,
        bytes: impl AsRef<[u8]>,
        id: Uuid,
    ) -> anyhow::Result<Self> {
        let component = wt::component::Component::new(&runtime.engine, &bytes)?;

        let (connection_tx, connection_rx) = mpsc::channel(10);
        let (output_tx, output_rx) = mpsc::channel(100);
        let (hook_request_tx, hook_request_rx) = mpsc::channel(1000);

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
                hook_request_tx,
                connection_rx: Some(connection_rx),
                hook_request_rx: Some(hook_request_rx),
                last_activity: Arc::new(AtomicU64::new(utils::now_as_secs())),
                app_activity: runtime.app_activity,
            },
        );

        store.epoch_deadline_async_yield_and_update(1);

        let instance = Bindings::instantiate_async(&mut store, &component, &runtime.linker).await?;

        Ok(Self {
            id,
            instance,
            store,
            output: Some(output_rx),
            cancel_token: CancellationToken::new(),
        })
    }

    pub fn start_inactive_shutdown_task(&mut self) {
        // Spawn a task to monitor inactivity and stop the container
        let last_activity = self.store.data().last_activity.clone();
        let token_clone = self.cancel_token.clone();
        tokio::spawn(async move {
            loop {
                const CHECK_INTERVAL: u64 = 5; // seconds
                const TIMEOUT: u64 = 60; // seconds

                time::sleep(Duration::from_secs(CHECK_INTERVAL)).await;

                // Check if the container has been inactive for more than TIMEOUT seconds
                if utils::now_as_secs() - last_activity.load(Ordering::Relaxed) > TIMEOUT {
                    tracing::info!("container is inactive for too long, stopping...");
                    token_clone.cancel();
                    break;
                }
            }
        });
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        tokio::select! {
            result = self.instance.call_run(&mut self.store) => {
                let inner_result = result?;
                return inner_result.map_err(|e| anyhow::anyhow!("container error: {e:?}"));
            }
            _ = self.cancel_token.cancelled() => {
                tracing::info!("container stopped due to inactivity");
            }
        }

        Ok(())
    }

    pub fn take_output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output.take()
    }

    pub fn pass_output(&mut self) {
        let mut output = self
            .take_output()
            .expect("output channel should be available");
        let container_id = self.id.clone();

        tokio::spawn(async move {
            while let Some(line) = output.recv().await {
                tracing::info!(
                    "{container_id} > {}",
                    serde_json::to_string(&line).unwrap_or_else(|_| line.clone())
                );
            }
        });
    }
}

impl ContainerData {
    pub fn update_last_activity(&self) {
        let now = utils::now_as_secs();
        self.app_activity.store(now, Ordering::Relaxed);
        self.last_activity.store(now, Ordering::Relaxed);
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
