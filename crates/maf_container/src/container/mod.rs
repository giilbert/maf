mod io;
mod limits;

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use io::ContainerStdoutFactory;
use schemas::typed::AppSchema;
use tokio::{
    sync::{mpsc, oneshot},
    time,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wasmtime as wt;
use wasmtime_wasi::IoView;

use crate::{
    ContainerRuntime, container::limits::ContainerResourceLimiter, interface::BoxedConnection,
    runtime::wasi::Bindings, utils, wasi::HookRequest,
};

/// An instance of user-written WASI code running in a sandboxed environment.
pub struct Container {
    pub room_id: Uuid,
    pub store: wt::Store<ContainerData>,
    pub cancel_token: CancellationToken,
    instance: Bindings,
    output: Option<mpsc::Receiver<String>>,
    shared: ContainerHandle,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("store", &self.store)
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

/// Contains shared data for the container, including container statistics and signals.
///
/// This struct is lightweight and can be cloned cheaply, allowing it to be passed around without
/// significant overhead.
#[derive(Debug, Clone)]
pub struct ContainerHandle {
    pub room_id: Uuid,
    pub runtime: ContainerRuntime,
    pub cancel_token: CancellationToken,
    pub resources: Arc<ContainerResourceStats>,
    connection_tx: mpsc::Sender<BoxedConnection>,
    hook_request_tx: mpsc::Sender<HookRequest>,
}

#[derive(Debug, Clone, Copy)]
pub struct ContainerResourceLimit {
    pub memory: usize,
    pub table: usize,
}

impl ContainerResourceLimit {
    // TODO: parse resource limit from app config file
    pub fn sensible_default() -> Self {
        Self {
            memory: 16 * 1024 * 1024, // 16 MiB
            table: 1000,              // 1000 entries
        }
    }
}

#[derive(Debug, Default)]
pub struct ContainerResourceStats {
    pub memory_usage: AtomicUsize,
    pub table_usage: AtomicUsize,
}

/// Data shared between the container and the host runtime.
///
/// Compared to `ContainerHandle`, this contains details intrinsic to the container's operation,
/// such as the WASI context, resource table, and actor communication channels. This struct is
/// heavier and should be used when the container's internal state is needed.
pub struct ContainerData {
    pub resources: wasmtime_wasi::ResourceTable,
    pub wasi_ctx: wasmtime_wasi::WasiCtx,
    pub connection_tx: mpsc::Sender<BoxedConnection>,
    pub hook_request_tx: mpsc::Sender<HookRequest>,
    pub connection_rx: Option<mpsc::Receiver<BoxedConnection>>,
    pub hook_request_rx: Option<mpsc::Receiver<HookRequest>>,

    pub app_schema_tx: Option<oneshot::Sender<AppSchema>>,
    pub app_schema_rx: Option<oneshot::Receiver<AppSchema>>,

    pub(crate) last_activity: Arc<AtomicU64>,
    pub(crate) app_activity: &'static AtomicU64,
    pub(crate) limiter: ContainerResourceLimiter,
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
        room_id: Uuid,
        resource_limit: ContainerResourceLimit,
    ) -> anyhow::Result<Self> {
        let component = wt::component::Component::new(&runtime.engine, &bytes)?;

        let (connection_tx, connection_rx) = mpsc::channel(10);
        let (output_tx, output_rx) = mpsc::channel(100);
        let (hook_request_tx, hook_request_rx) = mpsc::channel(1000);
        let (app_schema_tx, app_schema_rx) = oneshot::channel();

        let resource_stats = Arc::<ContainerResourceStats>::default();
        let cancel_token = CancellationToken::new();
        let shared = ContainerHandle {
            room_id,
            runtime: runtime.clone(),
            cancel_token: cancel_token.clone(),
            resources: resource_stats.clone(),
            connection_tx: connection_tx.clone(),
            hook_request_tx: hook_request_tx.clone(),
        };

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
                app_schema_rx: Some(app_schema_rx),
                app_schema_tx: Some(app_schema_tx),
                last_activity: Arc::new(AtomicU64::new(utils::now_as_secs())),
                app_activity: runtime.app_activity,
                limiter: ContainerResourceLimiter {
                    room_id,
                    stats: resource_stats.clone(),
                    limits: resource_limit,
                },
            },
        );

        store.limiter_async(|data| &mut data.limiter);
        store.epoch_deadline_async_yield_and_update(1);

        let instance = Bindings::instantiate_async(&mut store, &component, &runtime.linker).await?;

        Ok(Self {
            room_id,
            instance,
            store,
            output: Some(output_rx),
            cancel_token: cancel_token.clone(),
            shared,
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
                return inner_result.map_err(|_| anyhow::anyhow!("unknown container error"));
            }
            _ = self.cancel_token.cancelled() => {
                tracing::info!("container stopped due to inactivity");
            }
        }

        Ok(())
    }

    /// Dry-run the container without listening for IO events (connections, hooks, etc).
    ///
    /// This is useful for checking if the container can create the app without errors and report
    /// data for type generation.
    pub async fn dry_run(&mut self) -> anyhow::Result<()> {
        tokio::time::timeout(Duration::from_millis(100), async move {
            self.instance.call_dry_run(&mut self.store).await
        })
        .await
        .map_err(|_| anyhow::anyhow!("container dry-run timed out"))?
        .map_err(|e| anyhow::anyhow!("container error: {e:?}"))?
        .map_err(|_| anyhow::anyhow!("unknown container error"))?;

        Ok(())
    }

    pub fn get_app_schema(&mut self) -> anyhow::Result<oneshot::Receiver<AppSchema>> {
        let rx = self
            .store
            .data_mut()
            .app_schema_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("app schema receiver already taken"))?;

        Ok(rx)
    }

    pub fn take_output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output.take()
    }

    pub fn pass_output(&mut self) {
        let mut output = self
            .take_output()
            .expect("output channel should be available");
        let container_id = self.room_id.clone();

        tokio::spawn(async move {
            while let Some(line) = output.recv().await {
                tracing::info!(
                    "{container_id} > {}",
                    serde_json::to_string(&line).unwrap_or_else(|_| line.clone())
                );
            }
        });
    }

    #[inline]
    pub fn handle(&self) -> ContainerHandle {
        self.shared.clone()
    }
}

impl ContainerHandle {
    pub async fn add_connection(&self, connection: BoxedConnection) -> anyhow::Result<()> {
        self.connection_tx
            .send(connection)
            .await
            .map_err(|_| anyhow::anyhow!("failed to add connection to room {}", self.room_id))
    }

    pub async fn send_hook_request(&self, request: HookRequest) -> anyhow::Result<()> {
        self.hook_request_tx
            .send(request)
            .await
            .map_err(|_| anyhow::anyhow!("failed to send hook request to room {}", self.room_id))
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
