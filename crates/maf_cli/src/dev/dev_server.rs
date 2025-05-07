use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::{ErrorResponse, Response},
    routing::get,
};
use maf_container::{
    server::{handle_ws_upgrade, Bundle, Room},
    ContainerRuntime,
};
use notify::RecommendedWatcher;
use notify_debouncer_full::{
    new_debouncer_opt, DebounceEventResult, Debouncer, NoCache, RecommendedCache,
};
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
    room: RwLock<Room>,
    container_runtime: ContainerRuntime,
}

pub async fn start_dev_server(config: DevServerConfig) -> anyhow::Result<()> {
    let address = format!("0.0.0.0:{}", config.port);
    pretty::info!("starting maf dev server...");

    let runtime = ContainerRuntime::init()?;

    // This is so jank
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
            pretty::info!("reloading room...");

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

    let app = axum::Router::new()
        .route("/@/{org_slug}/{app_slug}/connect", get(connect_route))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(address).await?;

    pretty::info!("dev server listening on {}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn load_room(
    reload_notify: Arc<tokio::sync::Notify>,
    runtime: &ContainerRuntime,
    path: &str,
) -> anyhow::Result<Room> {
    let bundle = Bundle::load_wasm_module_from_file(path)?;
    let (room, mut container) = Room::new(&runtime, bundle).await?;

    let mut output = container.take_output().expect("failed to take output");
    let forward_output = async move {
        while let Some(line) = output.recv().await {
            let line = line.trim_end_matches(|s| s == '\n' || s == '\r' as char);
            println!("{} {}", ">".blue(), &line);
        }
    };

    let cancel_token = container.cancel_token.clone();
    let cancel_on_signal = async move {
        reload_notify.notified().await;
        cancel_token.cancel();
    };

    let run_container = async move {
        if let Err(e) = container.run().await {
            pretty::error!("failed to run container: {}", e);
            return;
        }

        pretty::info!("container exited");
    };

    tokio::spawn(forward_output);
    tokio::spawn(cancel_on_signal);
    tokio::spawn(run_container);

    pretty::info!("loaded room from {}", path);

    Ok(room)
}

fn create_file_watcher(
    path: &std::path::Path,
) -> anyhow::Result<(
    Arc<tokio::sync::Notify>,
    Debouncer<RecommendedWatcher, NoCache>,
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

    pretty::info!("watching for changes in {}", path.display());
    debouncer.watch(path, notify::RecursiveMode::NonRecursive)?;

    Ok((notify, debouncer))
}

async fn connect_route(
    State(state): State<DevServerState>,
    Path((org_slug, app_name)): Path<(String, String)>,
    ws: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let room = state.inner.room.read().await.clone();
    Ok(handle_ws_upgrade(ws, room).await)
}
