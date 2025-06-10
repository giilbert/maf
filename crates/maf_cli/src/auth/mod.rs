use clap::Subcommand;

use crate::{pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum AuthCommands {
    /// Add a authentication token to the CLI
    Login,
}

pub async fn handle_commands(context: &mut Context, command: AuthCommands) -> anyhow::Result<()> {
    match command {
        AuthCommands::Login => handle_login(context, command).await,
    }
}

async fn handle_login(context: &mut Context, _command: AuthCommands) -> anyhow::Result<()> {
    context.config.token = Some(
        rpassword::prompt_password(
            format!(
                "{} {} ",
                "?".bold().purple(),
                "Enter your authentication token:".bold()
            )
            .as_str(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to read token: {}", e))?,
    );

    context.config.save().await?;

    pretty::info!("Configuration saved successfully.");

    Ok(())
}
