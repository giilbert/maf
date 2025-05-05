mod admin;
mod app;
mod context;
mod dev;
mod input;
mod pretty;

use admin::AdminCommands;
use app::AppCommands;
use clap::{Parser, Subcommand};

pub use context::Context;
use dev::DevCommands;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Server management commands
    #[command(subcommand)]
    Admin(AdminCommands),
    #[command(subcommand)]
    App(AppCommands),
    Dev {
        #[arg(value_name = "FILE_PATH")]
        file_path: Option<String>,

        #[command(subcommand)]
        subcommand: Option<DevCommands>,
    },
}

async fn try_main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    let context = Context::new()?;

    match Cli::parse().commands {
        Commands::Admin(admin) => admin::handle_commands(&context, admin).await?,
        Commands::App(app) => app::handle_commands(&context, app).await?,
        Commands::Dev {
            file_path,
            subcommand,
        } => dev::handle_commands(&context, file_path, subcommand).await?,
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
