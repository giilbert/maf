mod dev_server;
mod platform;
mod rooms;
mod typed;

use std::{path::Path, process};

use clap::Subcommand;
use dev_server::DevServerConfig;

use crate::Context;

#[derive(Subcommand, Debug, Clone)]
pub enum DevCommands {
    Run { file_path: String },
}

pub async fn handle_commands(
    context: &mut Context,
    file_path: Option<String>,
    command: Option<DevCommands>,
    port: Option<u16>,
) -> anyhow::Result<()> {
    match command {
        Some(DevCommands::Run { file_path }) => handle_run(context, Some(file_path), port).await,
        None => {
            // Use the file path from the project config if available, otherwise use file_path
            let file_path = match context
                .project_config
                .as_ref()
                .map(|p| p.data.debug.output.clone())
            {
                Some(output) => output,
                None => file_path.ok_or_else(|| {
                    anyhow::anyhow!("No file path provided and no project config available")
                })?,
            };

            dev_server::start_local_server(
                context,
                DevServerConfig {
                    port: port.unwrap_or(DEFAULT_PORT),
                    wasm_module_path: file_path,
                    watch: true,
                },
            )
            .await
        }
    }
}

const DEFAULT_PORT: u16 = 1147; // Looks vaguely like Cobble

pub async fn handle_run(
    context: &mut Context,
    file_path: Option<String>,
    port: Option<u16>,
) -> anyhow::Result<()> {
    dev_server::start_local_server(
        context,
        DevServerConfig {
            port: port.unwrap_or(DEFAULT_PORT),
            wasm_module_path: match file_path {
                Some(path) => path,
                None => {
                    let project = context.assert_project();
                    run_build_command(&project.base, &project.data.debug.command)?;

                    let path =
                        std::fs::canonicalize(project.base.join(&project.data.debug.output))?;
                    path.to_string_lossy().to_string()
                }
            },
            watch: false,
        },
    )
    .await
}

pub fn run_build_command(base_path: &Path, command: &str) -> anyhow::Result<()> {
    print_dimmed!("[dev] Running build command `{}`", command);

    println!("\n");

    let start = std::time::Instant::now();
    let mut command = command.split(" ");
    let executable = command.next().expect("Command must have an executable");

    let args = command.collect::<Vec<_>>();

    let status = process::Command::new(executable)
        .args(args)
        .current_dir(&base_path)
        .spawn()?
        .wait()?;

    println!("\n");

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

    println!("\n");

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
