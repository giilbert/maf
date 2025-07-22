use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

use axum::{
    body::Body,
    extract::{Path, State, WebSocketUpgrade},
    response::Response,
    routing::{get, post},
};
use maf_container::{
    server::{handle_ws_upgrade, Bundle, ErrorResponse, RoomInner},
    wasi::bindings::{self, HookRequestCaller},
    ContainerRuntime,
};
use notify::RecommendedWatcher;
use notify_debouncer_full::{new_debouncer_opt, DebounceEventResult, Debouncer, RecommendedCache};
use tokio::sync::RwLock;

use crate::pretty;

#[derive(Debug)]
pub struct DevServerConfig {
    pub port: u16,
    pub wasm_module_path: String,
    pub watch: bool,
}

#[derive(Debug, Clone)]
struct DevServerState {
    inner: Arc<StateInner>,
}

#[derive(Debug)]
struct StateInner {
    room: RwLock<RoomInner>,
    container_runtime: ContainerRuntime,
}

pub async fn start_local_server(config: DevServerConfig) -> anyhow::Result<()> {
    let address = format!("0.0.0.0:{}", config.port);

    let runtime = ContainerRuntime::init(Box::leak(Box::new(AtomicU64::new(0))))?;

    // This is so jank

    // If the file watcher is enabled, we create a notify watcher that will trigger a reload
    // when the WASM file changes.
    //
    // If not, we just create a notify that will never trigger.
    let reload_notify = if config.watch {
        let (reload_notify, _watcher) = create_file_watcher(&std::fs::canonicalize(
            std::path::Path::new(&config.wasm_module_path),
        )?)?;

        reload_notify
    } else {
        Arc::new(tokio::sync::Notify::new())
    };

    let room = load_room(reload_notify.clone(), &runtime, &config.wasm_module_path).await?;

    let state = DevServerState {
        inner: Arc::new(StateInner {
            room: RwLock::new(room),
            container_runtime: runtime,
        }),
    };

    let state_clone = state.clone();
    let reload_room = async move {
        loop {
            reload_notify.notified().await;
            pretty::info!("[dev] Reloading room...");

            let room = load_room(
                reload_notify.clone(),
                &state_clone.inner.container_runtime,
                &config.wasm_module_path,
            )
            .await;

            match room {
                Ok(new_room) => {
                    let mut inner = state_clone.inner.room.write().await;
                    *inner = new_room;
                }
                Err(e) => {
                    pretty::error!("failed to reload room: {}", e);
                }
            }
        }
    };

    if config.watch {
        tokio::spawn(reload_room);
    }

    // Implement a subset of Platform APIs for the developer server
    let app = axum::Router::new()
        .route(
            "/@/{org_slug}/{app_slug}/{room_id}/connect",
            get(connect_route),
        )
        .route(
            "/@/{org_slug}/{app_slug}/{room_id}/hooks/{method}",
            post(hook_request_handler),
        )
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(address).await?;

    pretty::info!("Development server listening on {}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn load_room(
    reload_notify: Arc<tokio::sync::Notify>,
    runtime: &ContainerRuntime,
    path: &str,
) -> anyhow::Result<RoomInner> {
    let bundle = Bundle::load_wasm_module_from_file(path)?;
    let (room, mut container) = RoomInner::new(&runtime, bundle).await?;

    // Task to forward container output to the console
    let mut output = container.take_output().expect("failed to take output");
    let _forward_output = tokio::spawn(async move {
        while let Some(line) = output.recv().await {
            let line = line.trim_end_matches(|s| s == '\n' || s == '\r' as char);
            println!("{} {}", ">".blue(), &line);
        }
    });

    // When the reload notify is triggered, tell the container to stop
    let cancel_token = container.cancel_token.clone();
    let _cancel_on_signal = tokio::spawn(async move {
        reload_notify.notified().await;
        cancel_token.cancel();
    });

    let _run_container = tokio::spawn(async move {
        if let Err(e) = container.run().await {
            pretty::error!("[dev] Failed to run container: {}", e);
            return;
        }

        pretty::info!("[dev] Container exited");
    });

    pretty::info!("[dev] Loaded room from {}", path);

    Ok(room)
}

fn create_file_watcher(
    path: &std::path::Path,
) -> anyhow::Result<(
    Arc<tokio::sync::Notify>,
    Debouncer<RecommendedWatcher, RecommendedCache>,
)> {
    let notify = Arc::new(tokio::sync::Notify::new());

    let tx = notify.clone();
    let mut debouncer = new_debouncer_opt(
        Duration::from_secs(1),
        None,
        move |_res: DebounceEventResult| {
            tx.notify_waiters();
        },
        RecommendedCache::new(),
        notify::Config::default().with_compare_contents(true),
    )?;

    pretty::info!("[dev] Watching for changes in {}", path.display());
    debouncer.watch(path, notify::RecursiveMode::NonRecursive)?;

    Ok((notify, debouncer))
}

async fn connect_route(
    State(state): State<DevServerState>,
    Path((_org_slug, _app_name, _room_id)): Path<(String, String, String)>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let room = state.inner.room.read().await.clone();
    Ok(handle_ws_upgrade(ws, room).await)
}

async fn hook_request_handler(
    State(state): State<DevServerState>,
    Path((_org_slug, _app_name, room_id, method)): Path<(String, String, String, String)>,
) -> Result<Response, ErrorResponse> {
    if room_id != "default" {
        return Err(ErrorResponse::forbidden(Some(
            "only default room is supported for now",
        )));
    }

    let room = state.inner.room.read().await.clone();

    // TODO: handle hook bodies
    let response = room
        .call_hook(
            HookRequestCaller::Service,
            method.clone(),
            bindings::HookBody::None,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let response = match response {
        bindings::HookBody::None => Response::builder().body(Body::empty())?,
        bindings::HookBody::Json(json) => Response::builder()
            .header("Content-Type", "application/json")
            .body(Body::from(json))?,
    };

    Ok(response)
}
