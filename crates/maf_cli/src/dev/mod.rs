mod dev_server;

use clap::Subcommand;
use dev_server::DevServerConfig;

use crate::Context;

#[derive(Subcommand, Debug, Clone)]
pub enum DevCommands {
    Start,
}

pub async fn handle_commands(
    _context: &Context,
    file_path: Option<String>,
    command: Option<DevCommands>,
) -> anyhow::Result<()> {
    match command {
        Some(DevCommands::Start) => {
            todo!();
        }
        None => {
            let file_path = file_path.expect("FILE_PATH argument is required");

            dev_server::start_dev_server(DevServerConfig {
                port: 3000,
                wasm_module_path: file_path,
            })
            .await
        }
    }
}
