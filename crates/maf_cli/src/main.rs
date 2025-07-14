mod admin;
mod app;
mod auth;
mod config;
mod context;
mod dev;
mod init;
mod input;
mod pretty;

use admin::AdminCommands;
use app::AppCommands;
use clap::{Parser, Subcommand};

pub use context::Context;
use dev::DevCommands;

use crate::{auth::AuthCommands, config::ConfigCommands, init::InitOptions};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Run {
        /// Path to the WASM file to run
        file_path: Option<String>,
        /// Port to run the dev server on
        port: Option<u16>,
    },

    /// Server management commands
    #[command(subcommand)]
    Admin(AdminCommands),
    #[command(subcommand)]
    App(AppCommands),
    #[command(subcommand)]
    Auth(AuthCommands),
    #[command(subcommand)]
    Config(ConfigCommands),

    Dev {
        #[arg(value_name = "FILE_PATH")]
        file_path: Option<String>,

        #[arg(long, short, value_name = "PORT")]
        port: Option<u16>,

        #[command(subcommand)]
        subcommand: Option<DevCommands>,
    },

    Init(InitOptions),
}

async fn try_main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let mut context = Context::new().await?;

    match Cli::parse().commands {
        Commands::Run { file_path, port } => {
            dev::handle_run(&mut context, file_path, port).await?;
            return Ok(());
        }
        Commands::Admin(admin) => admin::handle_commands(&mut context, admin).await?,
        Commands::App(app) => app::handle_commands(&mut context, app).await?,
        Commands::Auth(auth) => auth::handle_commands(&mut context, auth).await?,
        Commands::Config(config) => config::handle_commands(&mut context, config).await?,
        Commands::Dev {
            file_path,
            subcommand,
            port,
        } => dev::handle_commands(&mut context, file_path, subcommand, port).await?,
        Commands::Init(options) => init::handle_init(options).await?,
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match try_main().await {
        Ok(_) => {}
        Err(e) => {
            pretty::error!("\n\n{:?}", e);
            if dotenvy::var("RUST_BACKTRACE").is_ok() {
                pretty::error!("Backtrace:\n{}", e.backtrace());
            }
            std::process::exit(1);
        }
    }
}
