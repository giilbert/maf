mod activity;
mod io;
mod limits;
pub mod meta;
pub mod runtime;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::Context;
use io::ContainerStdoutFactory;
use maf_schemas::apps::JsonMetaEntry;
use maf_schemas::typed::AppSchema;
use runtime::wasi::Bindings;
use tokio::sync::{mpsc, oneshot};
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use uuid::Uuid;
use wasmtime as wt;
use wasmtime::component::Component;
use wasmtime_wasi::{ResourceTable, WasiCtxView, WasiView};
use wasmtime_wasi_http::p2::body::HyperOutgoingBody;
use wasmtime_wasi_http::p2::types::{HostFutureIncomingResponse, OutgoingRequestConfig};
use wasmtime_wasi_http::p2::{
    HttpResult, WasiHttpCtxView, WasiHttpHooks, WasiHttpView, default_send_request,
};
use wasmtime_wasi_io::IoView;

use crate::container::activity::ActivityState;
use crate::container::limits::ContainerResourceLimiter;
use crate::container::meta::MetaStorage;
use crate::interface::BoxedConnection;
use crate::server::RoomHostImpl;
use crate::wasi::HookRequest;
use crate::wasi::bindings::AddKeyError;
use crate::{ContainerRuntime, utils};

type TakeableReceiver<T> = Option<mpsc::Receiver<T>>;
type TakeableOneshot<T> = Option<oneshot::Receiver<T>>;

#[derive(Debug)]
pub(super) struct AdditionalKeyRequest {
    key: String,
    response_tx: oneshot::Sender<Result<(), AddKeyError>>,
}

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
    /// Contains the resources and host-defined state for running the WASI code.
    store: wt::Store<ContainerData>,
    /// An interface to call functions in the WASI code, such as `run` and `dry_run`.
    instance: Bindings,
    /// Shared data between this struct and outside users of the container, such as the host and
    /// tasks that manage the container's lifecycle.
    shared: ContainerHandle,

    /// A channel for receiving output from the WASI code.
    output_rx: TakeableReceiver<String>,
    /// A channel for receiving additional room key requests from the WASI code. This is used to
    /// allow the WASI code to request additional keys for the room.
    additional_keys_rx: TakeableReceiver<AdditionalKeyRequest>,
    /// A channel for receiving the app schema from the WASI code. This is used to generate type
    /// information for the app, and is only used during the initial setup of the container.
    app_schema_rx: TakeableOneshot<AppSchema>,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("room_id", &self.room_id)
            .field("store", &self.store)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default)]
struct ContainerSignals {
    /// A cancellation token that can be used to signal the container to stop executing.
    ///
    /// This does not immediately stop the container, but signals it to stop at the next
    /// opportunity. Once the container has actually stopped, the `finished` token will be
    /// triggered. Once a cancellation token is triggered
    cancel: CancellationToken,
    /// A token that is triggered when the container has finished executing and cleaned up its
    /// resources. See the comment for `cancel_token` for more details and [`ContainerHandle::stop`]
    /// for usage.
    finished: CancellationToken,
    /// A token that is triggered when the container is ready to receive connections and other
    /// events. More specifically, this is when the container has submitted its first "wait for next
    /// connection" request to the host runtime.
    readied: CancellationToken,
}

/// Contains shared data for the container, including container statistics and signals.
///
/// This struct is lightweight and can be cloned cheaply, allowing it to be passed around without
/// significant overhead.
#[derive(Debug, Clone)]
pub struct ContainerHandle {
    room_id: Uuid,
    resources: Arc<ContainerResourceStats>,
    meta: MetaStorage,
    activity: Arc<ActivityState>,

    /// A channel for sending new connections to the WASI code running in the container. The user is
    /// [`runtime::wasi::FutureUserImpl`].
    connection_tx: mpsc::Sender<BoxedConnection>,
    /// A channel for sending hook requests to the WASI code running in the container. The user is
    /// [`runtime::wasi::FutureHookRequest`].
    hook_request_tx: mpsc::Sender<HookRequest>,

    signals: ContainerSignals,
}

impl ContainerHandle {
    /// Returns the room ID associated with this container.
    pub fn room_id(&self) -> Uuid {
        self.room_id
    }

    /// Stop executing container code and clean up resources associated with the container.
    ///
    /// This is not the same as just calling `cancel_token.cancel()`, which only signals the
    /// container to stop. This method will wait for the container to actually stop and clean up its
    /// resources, such as closing connections and releasing memory.
    ///
    /// TODO: graceful/user-defined shutdown?
    pub fn stop(&self) {
        self.activity.stop();
        self.signals.cancel.cancel();
    }

    /// Updates the last activity timestamp for the container to the current time.
    pub fn mark_activity(&self) {
        self.activity.record_activity(utils::now_as_secs());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ContainerResourceLimit {
    /// The maximum amount of memory (in bytes) that the container is allowed to use.
    pub memory: usize,
    /// The maximum number of entries in the resource table that the container is allowed to use.
    pub table: usize,
}

impl ContainerResourceLimit {
    // TODO: parse resource limit from app config file
    pub fn small_defaults() -> Self {
        Self {
            memory: 10 * 1024 * 1024, // 10 MiB
            table: 10_000,            // 10_000 entries
        }
    }
}

/// The maximum number of additional room keys that can be added via MAF WASI bindings.
pub const MAX_ADDITIONAL_KEYS: u8 = 1;

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
    resources: wasmtime_wasi::ResourceTable,
    wasi_ctx: wasmtime_wasi::WasiCtx,
    wasi_http_ctx: wasmtime_wasi_http::WasiHttpCtx,
    http_hooks: WasiHttpHooksData,
    meta: MetaStorage,

    /// A channel for sending hook requests to the WASI code running in the container. When room
    /// code calls `listen-user` (`maf.wit`) to start listening for new connections, the host
    /// runtime creates a [`runtime::wasi::FutureHookRequest`] and takes the receiving end of this
    /// channel.
    connection_rx: TakeableReceiver<BoxedConnection>,
    /// A channel for sending hook requests to the WASI code running in the container. When room
    /// code calls `listen-hook-request` (`maf.wit`) to start listening for new hook requests, the
    /// host runtime creates a [`runtime::wasi::FutureHookRequest`] and takes the receiving end of
    /// this channel.
    hook_request_rx: TakeableReceiver<HookRequest>,
    app_schema_tx: Option<oneshot::Sender<AppSchema>>,
    add_additional_keys_tx: mpsc::Sender<AdditionalKeyRequest>,

    /// The number of additional room keys that have been added to the room associated since this
    /// container was created. This is limited by [`MAX_ADDITIONAL_KEYS`].
    num_additional_keys: u8,
    signals: ContainerSignals,

    activity: Arc<ActivityState>,
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
    pub room_id: Uuid,
    /// The WebAssembly module bytes to load into the container.
    pub bytes: &'a [u8],
    pub resource_limit: ContainerResourceLimit,
    pub meta: Option<HashMap<String, JsonMetaEntry>>,
    pub secret: String,
}

impl Container {
    pub async fn load_from_binary<R: RoomHostImpl>(
        runtime: &super::ContainerRuntime<R>,
        options: CreateContainerOptions<'_>,
    ) -> anyhow::Result<Self> {
        let (connection_tx, connection_rx) = mpsc::channel::<BoxedConnection>(10);
        let (output_tx, output_rx) = mpsc::channel::<String>(100);
        let (add_additional_keys_tx, add_additional_keys_rx) =
            mpsc::channel::<AdditionalKeyRequest>(1);
        let (hook_request_tx, hook_request_rx) = mpsc::channel::<HookRequest>(100);
        let (app_schema_tx, app_schema_rx) = oneshot::channel::<AppSchema>();

        let meta = MetaStorage::new(options.meta.clone());
        let resource_stats = Arc::<ContainerResourceStats>::default();
        let resources = wasmtime_wasi::ResourceTable::default();
        let http_hooks = WasiHttpHooksData::new(&options);
        let stdout = ContainerStdoutFactory::new(output_tx.clone());
        let wasi_ctx = wasmtime_wasi::WasiCtx::builder().stdout(stdout).build();
        let wasi_http_ctx = wasmtime_wasi_http::WasiHttpCtx::new();

        let signals = ContainerSignals::default();
        let activity = Arc::new(ActivityState::new(utils::now_as_secs()));

        let shared = ContainerHandle {
            room_id: options.room_id,
            resources: resource_stats.clone(),
            meta: meta.clone(),
            activity: activity.clone(),

            connection_tx: connection_tx.clone(),
            hook_request_tx: hook_request_tx.clone(),

            signals: signals.clone(),
        };
        let limiter = ContainerResourceLimiter::new(
            shared.clone(),
            resource_stats.clone(),
            options.resource_limit,
        );

        let mut store = wt::Store::new(
            &runtime.engine,
            ContainerData {
                resources,
                wasi_ctx,
                wasi_http_ctx,
                http_hooks,
                meta,
                limiter,

                connection_rx: Some(connection_rx),
                hook_request_rx: Some(hook_request_rx),
                app_schema_tx: Some(app_schema_tx),
                add_additional_keys_tx,

                num_additional_keys: 0,

                signals: signals.clone(),
                activity,
                app_activity: runtime.app_activity,
            },
        );

        store.limiter_async(|data| &mut data.limiter);
        store.epoch_deadline_async_yield_and_update(1);

        let component = Component::new(&runtime.engine, options.bytes)?;
        let instance = Bindings::instantiate_async(&mut store, &component, &runtime.linker).await?;

        Ok(Self {
            room_id: options.room_id,
            instance,
            store,
            shared,

            app_schema_rx: Some(app_schema_rx),
            additional_keys_rx: Some(add_additional_keys_rx),
            output_rx: Some(output_rx),
        })
    }

    pub fn start_inactive_shutdown_task(&mut self) {
        // Spawn a task to monitor inactivity and stop the container
        let activity = self.store.data().activity.clone();
        let token_clone = self.shared.signals.cancel.clone();
        let finished = self.shared.signals.finished.clone();

        tokio::spawn(async move {
            loop {
                // TODO: let users configure the timeout via room config

                const CHECK_INTERVAL: u64 = 5; // seconds
                const TIMEOUT: u64 = 60; // seconds

                tokio::select! {
                    _ = time::sleep(Duration::from_secs(CHECK_INTERVAL)) => {}
                    // If the container has finished executing, we can stop the task.
                    _ = finished.cancelled() => break,
                }

                // Atomically check whether the container has been idle long enough and mark it
                // stopped in the same step so activity cannot sneak in between the comparison and
                // the shutdown transition.
                if activity.stop_if_inactive(utils::now_as_secs(), TIMEOUT) {
                    tracing::info!("container is inactive for too long, stopping...");
                    token_clone.cancel();
                    break;
                }
            }
        });
    }

    /// Run the container until it finishes executing or is cancelled.
    ///
    /// This method is meant to run in the background with other tasks, such as inserting the
    /// container into tracking structures and listening for connections. This means that we need a
    /// way to wait for the container to become ready: [`Container::ready`].
    pub async fn run_container<R: RoomHostImpl>(
        &mut self,
        runtime: ContainerRuntime<R>,
    ) -> anyhow::Result<()> {
        runtime.handle_additional_keys(
            self.handle(),
            self.additional_keys_rx
                .take()
                .context("additional_keys_rx already taken")?,
        )?;

        let result = tokio::select! {
            result = self.instance.call_run(&mut self.store) => {
                let inner_result = result?;
                inner_result.map_err(|_| anyhow::anyhow!("unknown container error"))
            }
            _ = self.shared.signals.cancel.cancelled() => {
                tracing::debug!("container cancelled");
                Ok(())
            }
        };

        self.do_cleanup().await;

        result
    }

    /// Dry-run the container without listening for IO events (connections, hooks, etc).
    ///
    /// This is useful for checking if the container can create the app without errors and report
    /// data for type generation.
    pub async fn dry_run(&mut self) -> anyhow::Result<()> {
        let result = tokio::select! {
            result = self.instance.call_dry_run(&mut self.store) => {
                let inner_result = result?;
                inner_result.map_err(|_| anyhow::anyhow!("unknown container error"))
            }
            _ = self.shared.signals.cancel.cancelled() => {
                tracing::debug!("container cancelled");
                Ok(())
            }
            _ = time::sleep(Duration::from_secs(10)) => {
                tracing::error!("container dry-run timed out");
                Err(anyhow::anyhow!("container dry-run timed out"))
            }
        };

        self.do_cleanup().await;

        result
    }

    async fn do_cleanup(&mut self) {
        // Mark the container as stopped before dropping any handles so late activity updates are
        // ignored instead of extending the shutdown timer after the room is already gone.
        self.shared.stop();

        // Drop the output channel to signal that ensure that an output channel cannot be created
        // after the container has been stopped.
        drop(self.output_rx.take());

        // We are finished! This marks that the container has finished executing and cleaned up its
        // resources, in case any other tasks are waiting for it to finish.
        self.shared.signals.finished.cancel();
    }

    /// Wait for the container to send its app schema through the channel.
    ///
    /// If this function is called more than once, it will return an error, since the channel can
    /// only be received once.
    pub async fn get_app_schema(&mut self) -> anyhow::Result<AppSchema> {
        let rx = self
            .app_schema_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("app schema receiver already taken"))?;

        Ok(rx.await?)
    }

    pub fn output(&mut self) -> Option<mpsc::Receiver<String>> {
        self.output_rx.take()
    }

    /// Consumes the container's output channel and forwards all output lines to tracing logs.
    pub fn pass_output_to_tracing(&mut self) {
        let mut output = self.output().expect("output channel should be available");
        let container_id = self.room_id;

        tokio::spawn(
            async move {
                while let Some(line) = output.recv().await {
                    tracing::trace!(
                        "out: {}",
                        serde_json::to_string(&line).unwrap_or_else(|_| line.clone())
                    );
                }
            }
            .instrument(tracing::trace_span!("container_output", container_id = ?container_id)),
        );
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

    /// Returns the last activity timestamp in seconds since the Unix epoch.
    pub fn last_activity(&self) -> u64 {
        self.store.data().activity.last_activity()
    }
}

impl ContainerHandle {
    pub async fn add_connection(&self, connection: BoxedConnection) -> anyhow::Result<()> {
        if self.activity.is_stopped() {
            anyhow::bail!("room {} is shutting down", self.room_id);
        }

        tokio::select! {
            result = self.connection_tx.send(connection) => {
                result.map_err(|_| anyhow::anyhow!("connection channel for room {} closed", self.room_id))
            }
            _ = self.signals.cancel.cancelled() => {
                anyhow::bail!("room {} is shutting down", self.room_id);
            }
        }
    }

    pub async fn send_hook_request(&self, request: HookRequest) -> anyhow::Result<()> {
        self.hook_request_tx
            .send(request)
            .await
            .map_err(|_| anyhow::anyhow!("failed to send hook request to room {}", self.room_id))
    }

    pub async fn wait_for_finish(&self) {
        self.signals.finished.cancelled().await;
    }

    /// Returns the time it took the container to become ready to receive connections and other
    /// events. Timeouts after the provided duration.
    pub async fn ready(&self, timeout: Duration) -> anyhow::Result<Duration> {
        let start = Instant::now();

        match tokio::time::timeout(timeout, self.signals.readied.cancelled()).await {
            Ok(()) => Ok(start.elapsed()),
            Err(_) => {
                anyhow::bail!(
                    "container {} did not become ready within the specified timeout of {:?}",
                    self.room_id,
                    timeout
                )
            }
        }
    }

    /// Gets the [`MetaStorage`] associated with this container, which can be used to read and write
    /// meta values associated with the room.
    pub fn meta(&self) -> &MetaStorage {
        &self.meta
    }

    /// Returns a struct containing information about the container's resource usage.
    pub fn resources(&self) -> &ContainerResourceStats {
        &self.resources
    }
}

impl ContainerData {
    pub fn update_last_activity(&self) {
        if self.activity.is_stopped() {
            return;
        }

        let now = utils::now_as_secs();
        self.app_activity.store(now, Ordering::Relaxed);
        self.activity.record_activity(now);
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

#[derive(Debug)]
pub struct WasiHttpHooksData {
    id: Uuid,
    secret: String,
}

impl WasiHttpHooksData {
    pub fn new(options: &CreateContainerOptions<'_>) -> Self {
        Self {
            id: options.room_id,
            secret: options.secret.clone(),
        }
    }
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
