use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Weak};

use anyhow::Context;
use colored::Colorize;
use maf_core::ContainerRuntime;
use maf_core::server::{App, Bundle, RoomHostImpl, RoomsStorage, UpgradeableRoomHostImpl};
use maf_schemas::apps::AppNameAndOrgSlug;
use maf_schemas::project_config::ProjectConfigFile;
use uuid::Uuid;

use crate::dev::dev_server::{DevServerBuildMode, StartDevServerConfig, StartDevServerMode};
use crate::pretty;

#[derive(Debug, Clone)]
pub struct DevServerHost(Arc<DevServerHostInner>);

/// A weak reference to a [`DevServerHost`], which can be used to avoid cyclic references when the
/// host needs to be stored in a room or other structure that is owned by the host. We also need
/// this to be able to create a [`RoomsStorage`] and store it in the host, since the host needs the
/// storage to be created first.
#[derive(Debug, Clone)]
pub struct WeakDevServerHost(Weak<DevServerHostInner>);

#[derive(Debug)]
struct DevServerHostInner {
    /// Determines how to load the project configuration and WebAssembly module for the development
    /// server.
    mode: StartDevServerMode,
    /// Manages the room state for the development server.
    rooms_storage: RoomsStorage<DevServerHost>,
    /// Runs WebAssembly modules for the development server.
    container_runtime: ContainerRuntime,
}

impl DevServerHost {
    pub async fn new(config: &StartDevServerConfig) -> anyhow::Result<Self> {
        /// A static variable to track the last activity time of the development server. This is
        /// unused but [`ContainerRuntime`] wants it. TODO: make optional
        static APP_ACTIVITY: AtomicU64 = AtomicU64::new(0);
        let container_runtime = ContainerRuntime::init(&APP_ACTIVITY)
            .context("failed to initialize container runtime")?;

        let inner = Arc::new_cyclic(|host| {
            let weak = WeakDevServerHost(host.clone());
            let rooms_storage = RoomsStorage::new(weak.clone());

            DevServerHostInner {
                mode: config.mode.clone(),
                rooms_storage,
                container_runtime,
            }
        });
        let this = Self(inner);

        Ok(this)
    }

    pub fn project_config(&self) -> ProjectConfigFile {
        match &self.0.mode {
            StartDevServerMode::RunProject { config, .. } => config.data.clone(),
            StartDevServerMode::RunWasmFile { .. } => ProjectConfigFile::default_anonymous(),
        }
    }

    /// Load the bundle for the project that the development server is running in.
    pub async fn load_default_bundle(&self) -> anyhow::Result<Bundle> {
        let path = match &self.0.mode {
            // When running a project, check the `maf-project.toml` file for the path to the *debug*
            // build of the WebAssembly module.
            StartDevServerMode::RunProject { config, build_mode } => {
                let target = match build_mode {
                    DevServerBuildMode::Debug => config.data.debug.as_ref(),
                    DevServerBuildMode::Release => config.data.release.as_ref(),
                    DevServerBuildMode::Skip => unreachable!(),
                };

                let target = target.ok_or_else(|| {
                    anyhow::anyhow!(
                        "no build configuration found for the current build mode ({:?})",
                        build_mode
                    )
                })?;

                config
                    .base
                    .join(&target.output)
                    .canonicalize()
                    .context("failed to canonicalize path to WebAssembly module")?
            }
            // When running a single WASM file, use that file directly.
            StartDevServerMode::RunWasmFile { file_path } => file_path.clone().into(),
        };

        let wasm_module = tokio::fs::read(&path)
            .await
            .context("failed to load the WebAssembly module")?;

        Bundle::from_wasm_bytes(self.project_config(), wasm_module.into())
    }
}

impl RoomHostImpl for DevServerHost {
    type WeakRef = WeakDevServerHost;

    fn weak(&self) -> Self::WeakRef {
        WeakDevServerHost(Arc::downgrade(&self.0))
    }

    fn container_runtime(&self) -> &maf_core::ContainerRuntime {
        &self.0.container_runtime
    }

    fn room_storage(&self) -> &RoomsStorage<Self> {
        &self.0.rooms_storage
    }

    fn update_last_activity(&self) -> anyhow::Result<()> {
        // No-op for development server, since we don't auto expire rooms or the server itself.
        Ok(())
    }

    async fn validate_api_key(
        &self,
        _app: &App,
        _headers: &axum::http::HeaderMap,
    ) -> anyhow::Result<bool> {
        static SHOWED_API_KEY_MESSAGE: std::sync::Once = std::sync::Once::new();
        SHOWED_API_KEY_MESSAGE.call_once(|| {
            pretty::info!("You are using an API that requires a service account API credentials. The development server does not check the validity of API credentials, so any API key will be accepted. In a production environment, this may not be the case. Please ensure that you are using valid API credentials in production.");
        });

        Ok(true)
    }

    async fn app(&self, id: AppNameAndOrgSlug) -> anyhow::Result<Option<App>> {
        let app = App::from_serialized(
            &id.app,
            &id.org,
            maf_schemas::apps::App {
                id: Uuid::nil(),
                name: id.app.clone(),
                config: Some(toml::to_string_pretty(&self.project_config()).unwrap()),
                api_client_id: "dev-server".to_string(),
                api_secret: "dev-server-secret".to_string(),
            },
        )?;

        Ok(Some(app))
    }

    async fn load_bundle_for_app(&self, _app: &App) -> anyhow::Result<Bundle> {
        self.load_default_bundle().await
    }

    async fn set_up_container_logging(
        &self,
        name: &str,
        container: &mut maf_core::Container,
    ) -> anyhow::Result<()> {
        let mut output = container.output().expect("container output not available");
        let id = name.to_string();

        tokio::spawn(async move {
            let mut line_buffer = String::new();

            while let Some(line) = output.recv().await {
                line_buffer.push_str(&line);

                // Drain the buffer until we have a full line to print.
                while let Some(pos) = line_buffer.find('\n') {
                    let line = line_buffer.drain(..=pos).collect::<String>();
                    println!("{} {}", format!("[{id}]").dimmed(), line.trim_end());
                }
            }
        });

        Ok(())
    }
}

impl UpgradeableRoomHostImpl<DevServerHost> for WeakDevServerHost {
    fn upgrade(&self) -> Option<DevServerHost> {
        self.0.upgrade().map(DevServerHost)
    }
}
