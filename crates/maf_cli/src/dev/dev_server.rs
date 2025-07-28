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
    ContainerResourceLimit, ContainerRuntime,
};
use notify::RecommendedWatcher;
use notify_debouncer_full::{new_debouncer_opt, DebounceEventResult, Debouncer, RecommendedCache};
use schemas::apps::RoomCreationStrategy;
use tokio::sync::RwLock;

use crate::{dev::rooms::DevRoomsStorage, pretty, print_dimmed, Context};

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
    runtime: ContainerRuntime,
    rooms: DevRoomsStorage,
}

pub async fn start_local_server(
    context: &mut Context,
    config: DevServerConfig,
) -> anyhow::Result<()> {
    if let Some(project_config) = context.project_config.as_ref() {
        print_dimmed!(
            "[dev] Read project config from {}",
            project_config.base.join("maf-project.toml").display()
        );
    }

    let room_creation_strategy = context
        .project_config
        .as_ref()
        .map(|c| c.data.rooms.clone())
        .unwrap_or(RoomCreationStrategy::AutoCreate);
    print_dimmed!(
        "[dev] Using room creation strategy: {}",
        room_creation_strategy.format_with_description()
    );

    // ContainerRuntime uses this variable to track whether the app is active. In dev mode, we
    // use this variable to store but not use the activity, since we don't need to auto stop.
    let app_activity = Box::leak(Box::new(AtomicU64::new(0)));
    let runtime = ContainerRuntime::init(app_activity)?;

    // This is so jank
    //
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
            runtime,
            rooms: DevRoomsStorage::new(context.project_config.clone()),
        }),
    };

    let state_clone = state.clone();
    let reload_room = async move {
        loop {
            reload_notify.notified().await;
            print_dimmed!("[dev] Reloading room...");

            let room = load_room(
                reload_notify.clone(),
                &state_clone.inner.runtime,
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

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    println!("[dev] Development server listening on {}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn load_room(
    reload_notify: Arc<tokio::sync::Notify>,
    runtime: &ContainerRuntime,
    path: &str,
) -> anyhow::Result<RoomInner> {
    let bundle = Bundle::load_wasm_module_from_file(path)?;
    let (room, mut container) = RoomInner::new(
        &runtime,
        bundle,
        ContainerResourceLimit {
            memory: usize::MAX,
            table: usize::MAX,
        },
    )
    .await?;

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

    print_dimmed!("[dev] Loaded room from {}", path);

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

    print_dimmed!("[dev] Watching for changes in {}", path.display());
    debouncer.watch(path, notify::RecursiveMode::NonRecursive)?;

    Ok((notify, debouncer))
}

async fn connect_route(
    State(state): State<DevServerState>,
    Path((_org_slug, _app_name, room_key)): Path<(String, String, String)>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let room = state
        .inner
        .rooms
        .get_by_key_or_id(&room_key)
        .await
        .ok_or_else(|| {
            ErrorResponse::not_found(Some(&format!(
                "Room with key or ID '{}' not found",
                room_key
            )))
        })?;

    Ok(handle_ws_upgrade(ws, room.inner.clone()).await)
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
