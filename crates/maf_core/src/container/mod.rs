mod io;
mod limits;
pub mod meta;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use io::ContainerStdoutFactory;
use maf_schemas::apps::JsonMetaEntry;
use maf_schemas::typed::AppSchema;
use tokio::sync::{mpsc, oneshot};
use tokio::time;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wasmtime as wt;
use wasmtime_wasi::{ResourceTable, WasiCtxView, WasiView};
use wasmtime_wasi_http::p2::body::HyperOutgoingBody;
use wasmtime_wasi_http::p2::types::{HostFutureIncomingResponse, OutgoingRequestConfig};
use wasmtime_wasi_http::p2::{
    HttpResult, WasiHttpCtxView, WasiHttpHooks, WasiHttpView, default_send_request,
};
use wasmtime_wasi_io::IoView;

use crate::container::limits::ContainerResourceLimiter;
use crate::container::meta::MetaStorage;
use crate::interface::BoxedConnection;
use crate::runtime::wasi::Bindings;
use crate::wasi::HookRequest;
use crate::{ContainerRuntime, utils};

/// An instance of user-written WASI code running in a sandboxed environment.
///
/// This struct manages the lifecycle of the container, including its creation, execution, and
/// shutdown. It also provides methods for interacting with the container, such as sending hook
/// requests and receiving output.
///
/// This is different from a "Room" in that it does not contain any logic related to managing
/// connections, room lifecycle, etc. It is purely the execution environment for the WASI code, and
/// can be used in different contexts.
pub struct Container {
    /// The room in which this container was created for, used for logging and identification
    /// purposes.
    room_id: Uuid,
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
    pub meta: MetaStorage,
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
    pub fn small_defaults() -> Self {
        Self {
            memory: 16 * 1024 * 1024, // 16 MiB
            table: 10_000,            // 10_000 entries
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
    pub wasi_http_ctx: wasmtime_wasi_http::WasiHttpCtx,
    pub connection_tx: mpsc::Sender<BoxedConnection>,
    pub hook_request_tx: mpsc::Sender<HookRequest>,
    pub connection_rx: Option<mpsc::Receiver<BoxedConnection>>,
    pub hook_request_rx: Option<mpsc::Receiver<HookRequest>>,
    pub http_hooks: WasiHttpHooksData,

    pub app_schema_tx: Option<oneshot::Sender<AppSchema>>,
    pub app_schema_rx: Option<oneshot::Receiver<AppSchema>>,

    pub(crate) meta: MetaStorage,
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

pub struct CreateContainerOptions<'a> {
    pub bytes: &'a [u8],
    pub resource_limit: ContainerResourceLimit,
    pub meta: Option<HashMap<String, JsonMetaEntry>>,
    pub secret: String,
}

impl Container {
    pub async fn load_from_binary(
        runtime: &super::ContainerRuntime,
        id: Uuid,
        options: CreateContainerOptions<'_>,
    ) -> anyhow::Result<Self> {
        let component = wt::component::Component::new(&runtime.engine, options.bytes)?;

        let (connection_tx, connection_rx) = mpsc::channel(10);
        let (output_tx, output_rx) = mpsc::channel(100);
        let (hook_request_tx, hook_request_rx) = mpsc::channel(1000);
        let (app_schema_tx, app_schema_rx) = oneshot::channel();

        let meta = MetaStorage::new(options.meta);
        let resource_stats = Arc::<ContainerResourceStats>::default();
        let cancel_token = CancellationToken::new();
        let shared = ContainerHandle {
            room_id: id,
            runtime: runtime.clone(),
            cancel_token: cancel_token.clone(),
            resources: resource_stats.clone(),
            connection_tx: connection_tx.clone(),
            hook_request_tx: hook_request_tx.clone(),
            meta: meta.clone(),
        };

        let resources = wasmtime_wasi::ResourceTable::default();
        let stdout = ContainerStdoutFactory::new(output_tx.clone());
        let wasi_ctx = wasmtime_wasi::WasiCtx::builder().stdout(stdout).build();
        let wasi_http_ctx = wasmtime_wasi_http::WasiHttpCtx::new();

        let mut store = wt::Store::new(
            &runtime.engine,
            ContainerData {
                resources,
                wasi_ctx,
                wasi_http_ctx,
                connection_tx,
                hook_request_tx,
                connection_rx: Some(connection_rx),
                hook_request_rx: Some(hook_request_rx),
                app_schema_rx: Some(app_schema_rx),
                app_schema_tx: Some(app_schema_tx),
                http_hooks: WasiHttpHooksData {
                    id,
                    secret: options.secret,
                },
                meta,
                last_activity: Arc::new(AtomicU64::new(utils::now_as_secs())),
                app_activity: runtime.app_activity,
                limiter: ContainerResourceLimiter {
                    room_id: id,
                    stats: resource_stats.clone(),
                    limits: options.resource_limit,
                },
            },
        );

        store.limiter_async(|data| &mut data.limiter);
        store.epoch_deadline_async_yield_and_update(1);

        let instance = Bindings::instantiate_async(&mut store, &component, &runtime.linker).await?;

        Ok(Self {
            room_id: id,
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

    pub async fn recv_app_schema(&mut self) -> anyhow::Result<AppSchema> {
        let rx = self
            .store
            .data_mut()
            .app_schema_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("app schema receiver already taken"))?;

        Ok(rx.await?)
    }

    pub fn output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output.take()
    }

    /// Consumes the container's output channel and forwards all output lines to tracing logs.
    pub fn pass_output(&mut self) {
        let mut output = self.output().expect("output channel should be available");
        let container_id = self.room_id;

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

    /// Returns the room ID associated with this container, which is used for logging and
    /// identification purposes.
    pub fn room_id(&self) -> Uuid {
        self.room_id
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

impl WasiView for ContainerData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resources,
        }
    }
}

impl IoView for ContainerData {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resources
    }
}

impl WasiHttpView for ContainerData {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.wasi_http_ctx,
            table: &mut self.resources,
            hooks: &mut self.http_hooks,
        }
    }
}

pub struct WasiHttpHooksData {
    id: Uuid,
    secret: String,
}

impl WasiHttpHooks for WasiHttpHooksData {
    fn send_request(
        &mut self,
        mut request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse> {
        // Insert a header allowing the server to identify the room making the request and verify
        // its authenticity. The header is in the format `X-Maf-Id: room:<id>:<secret>`.
        request.headers_mut().insert(
            "X-Maf-Id",
            format!("room:{}:{}", self.id, self.secret).parse().unwrap(),
        );

        Ok(default_send_request(request, config))
    }
}
