mod global_config;
mod project_config;

use clap::Subcommand;
use colored::Colorize;
pub use global_config::GlobalConfig;
pub use project_config::ProjectConfig;

use crate::{pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommands {
    /// Show all configuration settings.
    Show,
    /// Set a configuration settings.
    Set {
        /// The configuration key to set. Run `maf config show` to see available keys.
        key: String,
        /// The value to set for the configuration key.
        value: String,
    },
    /// Reset a configuration setting to its default value.
    Reset {
        /// The key of the configuration setting to reset. Run `maf config show` to see available keys.
        key: String,
    },
}

pub fn handle_commands(context: &mut Context, command: ConfigCommands) -> anyhow::Result<()> {
    match command {
        ConfigCommands::Show => handle_show(context),
        ConfigCommands::Set { key, value } => handle_set(context, key, value),
        ConfigCommands::Reset { key } => handle_reset(context, key),
    }
}

fn handle_show(context: &mut Context) -> anyhow::Result<()> {
    let config = context.global_config.clone();
    println!(
        "{}",
        format!(
            "Global config loaded from {}",
            GlobalConfig::get_config_file()?.display()
        )
        .dimmed()
    );

    for (key, name, value) in [
        (
            "server_url",
            "Server URL",
            config.server_url.as_deref().map(|url| url.to_string()),
        ),
        (
            "token",
            "Token",
            config
                .token
                .as_deref()
                .map(|t| t[0..5].to_string() + &".".repeat(t.len() - 5)),
        ),
    ] {
        println!(
            "{} {}: {}",
            name.bold(),
            format!("`{}`", key).dimmed(),
            value.unwrap_or_else(|| "<not set>".dimmed().to_string())
        )
    }

    Ok(())
}

fn handle_set(context: &mut Context, key: String, value: String) -> anyhow::Result<()> {
    match key.as_str() {
        "server_url" => {
            if !value.starts_with("http://") && !value.starts_with("https://") {
                return Err(anyhow::anyhow!(
                    "Server URL must start with 'http://' or 'https://'"
                ));
            }
            context.global_config.server_url = Some(value);
        }
        "token" => {
            context.global_config.token = Some(value);
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
        }
    }

    context.global_config.save()?;
    pretty::info!("Configuration updated successfully.");

    Ok(())
}

fn handle_reset(context: &mut Context, key: String) -> anyhow::Result<()> {
    match key.as_str() {
        "server_url" => {
            context.global_config.server_url = None;
        }
        "token" => {
            context.global_config.token = None;
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
        }
    }

    context.global_config.save()?;
    pretty::info!("Configuration reset successfully.");

    Ok(())
}
