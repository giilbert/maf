use anyhow::Context as _;
use clap::Subcommand;
use colored::Colorize;
use uuid::Uuid;

use crate::Context;

#[derive(Subcommand, Debug, Clone)]
pub enum AppCommands {
    /// List, add, remove, or update users
    Create,
}

pub async fn handle_commands(context: &Context, command: AppCommands) -> anyhow::Result<()> {
    match command {
        AppCommands::Create => create_app(context).await,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAppParams {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub permissions: String,
}

pub async fn create_app(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let app = context
        .post::<()>(
            "/api/apps",
            &serde_json::json!({
                "name": "test_app",
            }),
        )
        .await
        .context("failed to create app")?;

    Ok(())
}
