mod global_config;

use clap::Subcommand;
use colored::Colorize;
pub use global_config::GlobalConfig;

use crate::{pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommands {
    /// Show all configuration settings
    Show,
    /// Set a configuration settings
    Set {
        /// The configuration key to set
        key: String,
        /// The value to set for the configuration key
        value: String,
    },
    /// Reset a configuration setting to its default value
    Reset {
        /// The configuration key to reset
        key: String,
    },
}

pub async fn handle_commands(context: &mut Context, command: ConfigCommands) -> anyhow::Result<()> {
    match command {
        ConfigCommands::Show => handle_show(context).await,
        ConfigCommands::Set { key, value } => handle_set(context, key, value).await,
        ConfigCommands::Reset { key } => handle_reset(context, key).await,
    }
}

async fn handle_show(context: &mut Context) -> anyhow::Result<()> {
    let config = context.config.clone();
    println!("{}", "Current configuration:".bold());

    for (key, value) in [
        (
            "Server URL",
            config.server_url.as_deref().map(|url| url.to_string()),
        ),
        (
            "Token",
            config
                .token
                .as_deref()
                .map(|t| t[0..5].to_string() + &".".repeat(t.len() - 5)),
        ),
    ] {
        match value {
            Some(value) => println!("{}: {}", key.bold(), value),
            None => println!("{}: {}", key.bold(), "<not set>".dimmed()),
        }
    }

    Ok(())
}

async fn handle_set(context: &mut Context, key: String, value: String) -> anyhow::Result<()> {
    match key.as_str() {
        "server_url" => {
            if !value.starts_with("http://") && !value.starts_with("https://") {
                return Err(anyhow::anyhow!(
                    "Server URL must start with 'http://' or 'https://'"
                ));
            }
            context.config.server_url = Some(value);
        }
        "token" => {
            context.config.token = Some(value);
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
        }
    }

    context.config.save().await?;
    pretty::info!("Configuration updated successfully.");
    Ok(())
}

async fn handle_reset(context: &mut Context, key: String) -> anyhow::Result<()> {
    match key.as_str() {
        "server_url" => {
            context.config.server_url = None;
        }
        "token" => {
            context.config.token = None;
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
        }
    }

    context.config.save().await?;
    pretty::info!("Configuration reset successfully.");
    Ok(())
}
