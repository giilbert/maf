mod dev_server;
mod host;
mod typed;

use std::path::Path;
use std::process;

use dev_server::StartDevServerConfig;

use crate::Context;
use crate::dev::dev_server::{DevServerBuildMode, StartDevServerMode};

// TODO: watch mode implementation?

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:1147"; // Looks vaguely like MAF

#[derive(Debug, Clone, clap::Args)]
pub struct RunCommand {
    /// Path to the WASM file to run. If not provided, the development server will look for a
    /// `maf-project.toml` file in the current directory and run the project defined in that
    /// file.
    pub file: Option<String>,
    /// Address and port to bind the development server to.
    #[arg(long, short, default_value_t = DEFAULT_ADDRESS.to_string())]
    pub address: String,
    /// Whether to use the release mode settings when building the project. If not provided, the
    /// development server will use the debug mode settings.
    #[arg(long, default_value_t = false)]
    pub release: bool,
}

pub async fn handle_run(context: &mut Context, args: RunCommand) -> anyhow::Result<()> {
    let project = context.assert_project();
    let build_mode = if args.release {
        DevServerBuildMode::Release
    } else {
        DevServerBuildMode::Debug
    };

    // How we run the development server depends on whether the user provided a WASM file to run or
    // not. This will probably change in the future.
    let mode = match args.file {
        Some(file_path) => StartDevServerMode::RunWasmFile { file_path },
        None => StartDevServerMode::RunProject {
            config: project.clone(),
            build_mode,
        },
    };

    dev_server::start_local_server(
        context,
        StartDevServerConfig {
            mode,
            address: args.address,
            build: build_mode,
        },
    )
    .await
}

pub fn run_build_command(base_path: &Path, command: &str) -> anyhow::Result<()> {
    print_dimmed!("[dev] Running build command `{}`", command);

    let start = std::time::Instant::now();
    let mut command = command.split(" ");
    let executable = command.next().expect("Command must have an executable");

    let args = command.collect::<Vec<_>>();

    let status = process::Command::new(executable)
        .args(args)
        .current_dir(base_path)
        .spawn()?
        .wait()?;

    if !status.success() {
        println!(
            "{}",
            format!(
                "[dev] Build command failed with status code: {}",
                status
                    .code()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "<unknown>".to_string())
            )
            .red()
        );
        std::process::exit(1);
    }

    print_dimmed!("[dev] Build completed in {:.2?}", start.elapsed());

    Ok(())
}

#[macro_export]
macro_rules! print_dimmed {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        println!("{}", format!($($arg)*).dimmed());
    };
}

pub use print_dimmed;
