mod dev_server;

use clap::Subcommand;
use dev_server::DevServerConfig;

use crate::Context;

#[derive(Subcommand, Debug, Clone)]
pub enum DevCommands {}

pub async fn handle_commands(
    _context: &Context,
    command: Option<DevCommands>,
) -> anyhow::Result<()> {
    match command {
        None => {
            dev_server::start_dev_server(DevServerConfig {
                port: 3000,
                wasm_module_path: "path/to/wasm/module".to_string(),
            })
            .await
        }
    }
}
