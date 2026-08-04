//! An implementation of a development server for running and testing MAF applications locally.
//!
//! This largely hosts code from the `maf_core` crate, but adds some development-specific features.

use anyhow::Context as _;
use colored::Colorize;
use maf_core::server::RoomHostImpl;
use maf_core::{Container, ContainerResourceLimit, CreateContainerOptions};
use maf_schemas::project_config::TypedConfig;
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::config::ProjectConfig;
use crate::dev::host::DevServerHost;
use crate::dev::{run_build_command, typed};
use crate::{Context, pretty, print_dimmed};

/// The mode in which the development server should run.
///
/// We make the distinction between running a single WASM file and running a project because the
/// latter has a `maf-project.toml` file that contains configuration for the project, while the
/// former does not. In `RunWasmFile` mode, we will not be able to load the `maf-project.toml` file,
/// and will assume the following default configs:
///
/// - Rooms will be unauthenticated (no API key required to connect to the room).
/// - One room will be autocreated and everyone will be put into that room.
#[derive(Debug, Clone)]
pub enum StartDevServerMode {
    RunWasmFile {
        file_path: String,
    },
    RunProject {
        config: ProjectConfig,
        build_mode: DevServerBuildMode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevServerBuildMode {
    /// Do not run the build command before starting the development server.
    Skip,
    /// Run the "debug" mode build command.
    Debug,
    /// Run the "release" mode build command.
    Release,
}

#[derive(Debug)]
pub struct StartDevServerConfig {
    /// The mode in which the development server should run.
    pub mode: StartDevServerMode,
    /// The address and port to bind the development server to.
    pub address: String,
    /// Whether the build command should be run before starting the development server.
    pub build: DevServerBuildMode,
}

pub async fn start_local_server(
    context: &mut Context,
    config: StartDevServerConfig,
) -> anyhow::Result<()> {
    print_dimmed!("[dev] Hello world! Starting development server...");

    if let Some(project_config) = &context.project_config
        && config.build != DevServerBuildMode::Skip
    {
        let target_config = match config.build {
            DevServerBuildMode::Debug => project_config.data.debug.as_ref(),
            DevServerBuildMode::Release => project_config.data.release.as_ref(),
            DevServerBuildMode::Skip => unreachable!(),
        };

        if let Some(target_config) = target_config {
            run_build_command(&project_config.base, &target_config.command)
                .context("failed to run build command")?;
        } else {
            pretty::warn!(
                "No build command found for {:?} mode. Skipping build.",
                config.build
            );
        }
    }

    let dev_server_host = DevServerHost::new(&config)
        .await
        .context("failed to create development server")?;

    // See if typed is enabled, then generate types for the project.
    if let Some(project_config) = &context.project_config
        && let Some(typed_config) = &project_config.data.typed
        && config.build == DevServerBuildMode::Debug
    {
        generate_types(&dev_server_host, project_config, typed_config)
            .await
            .context("failed to generate types for project")?;
    }

    let router = maf_core::server::create_router(&dev_server_host).with_state(dev_server_host);

    let addr = config
        .address
        .parse::<std::net::SocketAddr>()
        .context("failed to parse address")?;
    let listener = TcpListener::bind(&addr)
        .await
        .context("failed to bind to address")?;

    println!("[dev] Development server running at {}", addr);

    axum::serve(listener, router)
        .await
        .context("failed to start development server")?;

    Ok(())
}

async fn generate_types(
    state: &DevServerHost,
    project: &ProjectConfig,
    config: &TypedConfig,
) -> anyhow::Result<()> {
    let start = std::time::Instant::now();

    let bundle = state.load_default_bundle().await?;
    let mut container = Container::load_from_binary(
        state.container_runtime(),
        CreateContainerOptions {
            room_id: Uuid::nil(),
            bytes: bundle.wasm_module_bytes(),
            resource_limit: ContainerResourceLimit::small_defaults(),
            meta: None,
            // We don't care about the secret here, since we don't validate API requests in the dev
            // server.
            secret: "".to_string(),
        },
    )
    .await?;

    // We need to run the code in the container in order for it to report its schema and send it
    // back to us (through the channel).
    container.dry_run().await?;
    let schema = container.get_app_schema().await?;
    tracing::debug!("{}", format!("app schema received: {schema:?}").dimmed());

    typed::create_types_file_for_project(project, config, schema).await?;

    let elapsed = start.elapsed();
    println!(
        "{}",
        format!(
            "[dev] Types generated to `{}` in {:?}",
            config.out.display(),
            elapsed
        )
        .dimmed()
    );

    Ok(())
}
