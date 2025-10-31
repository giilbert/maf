use clap::Subcommand;

use crate::{pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum AuthCommands {
    /// Add a authentication token to the CLI
    Login,
    /// Logout from the CLI
    Logout,
}

pub fn handle_commands(context: &mut Context, command: AuthCommands) -> anyhow::Result<()> {
    match command {
        AuthCommands::Login => handle_login(context, command),
        AuthCommands::Logout => handle_logout(context, command),
    }
}

fn handle_login(context: &mut Context, _command: AuthCommands) -> anyhow::Result<()> {
    if context.global_config.token.is_some() {
        pretty::warn!("You are already logged in. Use `cobble auth logout` to log out first.");
        return Ok(());
    }

    context.global_config.token = Some(
        dialoguer::Password::new()
            .with_prompt(
                format!(
                    "{} {}",
                    "?".bold().purple(),
                    "Enter your authentication token".bold()
                )
                .as_str(),
            )
            .interact()
            .map_err(|e| anyhow::anyhow!("Failed to read token: {}", e))?,
    );

    context.global_config.save()?;

    pretty::info!("Configuration saved successfully.");

    Ok(())
}

fn handle_logout(context: &mut Context, _command: AuthCommands) -> anyhow::Result<()> {
    if context.global_config.token.is_none() {
        pretty::warn!("You are not logged in. Use `cobble auth login` to log in first.");
        return Ok(());
    }

    context.global_config.token = None;
    context.global_config.save()?;

    pretty::info!("Logged out successfully.");

    Ok(())
}
