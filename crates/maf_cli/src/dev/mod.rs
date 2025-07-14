mod dev_server;

use std::path::Path;

use clap::Subcommand;
use dev_server::DevServerConfig;

use crate::{pretty, Context};

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
            let file_path = file_path.expect("FILE_PATH argument is required");

            dev_server::start_dev_server(DevServerConfig {
                port: port.unwrap_or(DEFAULT_PORT),
                wasm_module_path: file_path,
                watch: true,
            })
            .await
        }
    }
}

const DEFAULT_PORT: u16 = 1147; // Looks vaguely like MAF

pub async fn handle_run(
    context: &mut Context,
    file_path: Option<String>,
    port: Option<u16>,
) -> anyhow::Result<()> {
    dev_server::start_dev_server(DevServerConfig {
        port: port.unwrap_or(DEFAULT_PORT),
        wasm_module_path: match file_path {
            Some(path) => path,
            None => {
                let project = context.assert_project();

                run_build_command(&project.base_path, &project.data.debug.command).await?;

                let path =
                    tokio::fs::canonicalize(project.base_path.join(&project.data.debug.output))
                        .await?;

                path.to_string_lossy().to_string()
            }
        },
        watch: false,
    })
    .await
}

pub async fn run_build_command(base_path: &Path, command: &str) -> anyhow::Result<()> {
    pretty::info!(
        "running build command `{}` in `{}`...",
        command,
        base_path.to_string_lossy()
    );

    println!("\n----------\n");

    let start = std::time::Instant::now();

    let mut command = command.split(" ");
    let executable = command.next().expect("Command must have an executable");

    let args = command.collect::<Vec<_>>();

    let mut process = tokio::process::Command::new(executable)
        .args(args)
        .current_dir(&base_path)
        .spawn()?;

    process.wait().await?;

    println!("\n----------\n");

    pretty::info!("build completed in {:.2?}", start.elapsed());

    Ok(())
}
