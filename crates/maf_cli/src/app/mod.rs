mod models;

use anyhow::Context as _;
use clap::Subcommand;
use colored::Colorize;
use uuid::Uuid;

use crate::{input::input, pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum AppCommands {
    List,
    Create,
    Delete { name: String },
}

pub async fn handle_commands(context: &Context, command: AppCommands) -> anyhow::Result<()> {
    match command {
        AppCommands::List => list_apps(context).await,
        AppCommands::Create => create_app(context).await,
        AppCommands::Delete { name } => delete_app(context, name).await,
    }
}

async fn list_apps(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let apps = context.get::<Vec<models::App>>("/api/apps").await?;

    if apps.is_empty() {
        println!("No apps found")
    } else {
        println!("Apps ({}):", apps.len());
        for app in apps {
            println!("- {}", app.name);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAppParams {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub permissions: String,
}

async fn create_app(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let name = input!(
        transform: |name: String| {
            if name.is_empty() {
                anyhow::bail!("Name cannot be empty.")
            }
            if name.len() > 100 {
                anyhow::bail!("Name cannot be longer than 100 characters.")
            }
            if !name
                .chars()
                .all(|c| (c.is_ascii_alphanumeric() && (c.is_ascii_lowercase() || c.is_numeric())) || c == '-')
            {
                anyhow::bail!("Name can only contain lowercase alphanumeric characters and hyphens.")
            }

            Ok(name)
        },
        "{} {}:",
        "Name".bold(),
        "(Lowercase alphanumeric characters and hyphens)".dimmed()
    );

    let app = context
        .post::<models::App>(
            "/api/apps",
            &serde_json::json!({
                "name": name
            }),
        )
        .await
        .context("failed to create app")?;

    println!("App `{}` created!", app.name);

    Ok(())
}

async fn delete_app(context: &Context, name: String) -> anyhow::Result<()> {
    context.assert_token();

    println!("Fetching app `{name}`...\n");

    let app = context
        .get::<models::App>(format!("/api/apps/{name}"))
        .await
        .context("failed to get app")?;

    println!(
        "{} {}",
        app.name.bold(),
        format!("(id: {})", app.id).dimmed()
    );

    println!("");

    println!(
        "{}",
        "Deleting an app will permanently remove all data associated with it."
            .red()
            .bold(),
    );
    let confirm: String = input!(
        "Are you sure you want to delete `{name}`? {}",
        "(`yes` to confirm)".dimmed()
    );

    if confirm != "yes" {
        pretty::error!("Aborted");
        return Ok(());
    }

    println!("Deleting app `{name}`...");

    context
        .delete::<models::App>(format!("/api/apps/{name}"), ())
        .await?;

    Ok(())
}
