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
use colored::Colorize;
pub use context::Context;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{fmt, EnvFilter};

use crate::auth::AuthCommands;
use crate::config::ConfigCommands;
use crate::init::InitOptions;

#[derive(Parser, Debug)]
#[command(version, about = include_str!("./about.txt"), long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Quickly execute a WASM file.
    Run {
        /// Path to the WASM file to run.
        file: Option<String>,
        /// Port to run the server on.
        #[arg(long, short, value_name = "PORT")]
        port: Option<u16>,
    },

    /// Utilities to manage MAF servers for administrators.
    #[command(subcommand)]
    Admin(AdminCommands),
    /// Manage MAF applications, deployments, and versions.
    #[command(subcommand)]
    App(AppCommands),
    /// Manage authentication tokens for MAF servers or MAF Platform.
    #[command(subcommand)]
    Auth(AuthCommands),
    /// Change CLI configuration.
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Initialize a new MAF application in the current directory.
    Init(InitOptions),
    /// Create a new MAF application prompts to customize it.
    Create(InitOptions),
}

async fn try_main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let mut context = Context::new()?;

    match Cli::parse().commands {
        Commands::Run {
            file: file_path,
            port,
        } => dev::handle_run(&mut context, file_path, port).await?,
        Commands::Admin(admin) => admin::handle_commands(&mut context, admin).await?,
        Commands::App(app) => app::handle_commands(&mut context, app).await?,
        Commands::Auth(auth) => auth::handle_commands(&mut context, auth)?,
        Commands::Config(config) => config::handle_commands(&mut context, config)?,
        // Commands::Dev {
        //     file: file_path,
        //     subcommand,
        //     port,
        // } => dev::handle_commands(&mut context, file_path, subcommand, port).await?,
        Commands::Init(options) => init::handle_init(options)?,
        Commands::Create(options) => init::handle_create(options)?,
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    match try_main().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!();
            eprintln!("{}\n", "A fatal error occured! :(".bold().red());
            eprintln!("{}", format!("{e:?}").red());
            std::process::exit(1);
        }
    }
}
