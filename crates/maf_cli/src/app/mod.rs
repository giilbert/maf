mod deploy;

use std::path::PathBuf;

use anyhow::Context as _;
use clap::Subcommand;
use colored::Colorize;
use maf_schemas::apps::CreateUserAppRequest;

use crate::{input::input, pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum AppCommands {
    /// List all apps in the current organization
    List,
    /// Create a new app
    Create,
    /// Get service account credentials for an app by name, or the current project's app if no name
    /// is provided
    Credentials { name: Option<String> },
    /// Show information about an app by name, or the current project's app if no name is provided
    View { name: Option<String> },
    /// Delete an app by name, or the current project's app if no name is provided
    Delete { name: Option<String> },
    /// Deploy an app by name and bundle, or the current project's app if neither is provided
    Deploy {
        /// The name of the app to deploy to
        #[clap(requires = "wasm_module_path")]
        name: Option<String>,
        /// Where to the find the WASM module to deploy
        wasm_module_path: Option<PathBuf>,
    },
}

pub async fn handle_commands(context: &mut Context, command: AppCommands) -> anyhow::Result<()> {
    match command {
        AppCommands::List => list_apps(context).await,
        AppCommands::Create => create_app(context).await,
        AppCommands::Credentials { name } => match name {
            Some(name) => get_app_credentials(context, &name).await,
            None => {
                let project = context.assert_project();
                get_app_credentials(context, &project.data.name).await
            }
        },
        AppCommands::View { name } => match name {
            Some(name) => view_app(context, &name).await,
            None => {
                let project = context.assert_project();
                view_app(context, &project.data.name).await
            }
        },
        AppCommands::Delete { name } => match name {
            Some(name) => delete_app(context, name).await,
            None => {
                let project = context.assert_project();
                delete_app(context, project.data.name.clone()).await
            }
        },
        AppCommands::Deploy {
            name,
            wasm_module_path,
        } => deploy::handle_deploy(context, name, wasm_module_path).await,
    }
}

async fn list_apps(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let apps = context
        .get::<Vec<maf_schemas::apps::App>>("/api/v1/apps")
        .await?;

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

async fn show_short_app_info(
    context: &Context,
    name: &str,
    silent: bool,
) -> anyhow::Result<maf_schemas::apps::App> {
    context.assert_token();

    if !silent {
        println!("Fetching app `{name}`...\n");
    }

    let app = context
        .get::<maf_schemas::apps::App>(format!("/api/v1/apps/{name}"))
        .await
        .with_context(|| format!("Failed to get app `{name}`"))?;

    println!(
        "{} {}",
        app.name.bold(),
        format!("(id: {})", app.id).dimmed()
    );
    println!();

    Ok(app)
}

async fn view_app(context: &Context, name: &str) -> anyhow::Result<()> {
    context.assert_token();

    let app = show_short_app_info(context, name, false).await?;

    println!("{}", "Configuration".bold());
    println!(
        "{}",
        app.config
            .unwrap_or("<none>".to_string().dimmed().to_string())
    );

    Ok(())
}

async fn create_app(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let config: CreateUserAppRequest = match context.project_config.as_ref() {
        Some(config) => {
            pretty::info!("Found existing project configuration. This will create a new app in the current organization.");

            println!();
            println!("{}: {}", "Name".bold(), config.data.name);
            println!(
                "{}: {}",
                "Room Creation Strategy".bold(),
                config.data.rooms.format_with_description()
            );
            println!();

            let success = dialoguer::Confirm::new()
                .with_prompt("Do you want to create a new app with these options?")
                .default(false)
                .interact()
                .context("Failed to read confirmation")?;

            if !success {
                println!();
                pretty::error!("Aborted");
                std::process::exit(0);
            }

            CreateUserAppRequest {
                name: config.data.name.clone(),
                config: Some(toml::to_string_pretty(&config.data)?),
            }
        }
        None => {
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

            CreateUserAppRequest {
                name: name.clone(),
                config: None, // Default to None, can be set later
            }
        }
    };

    let app = context
        .post::<maf_schemas::apps::App>("/api/v1/apps", &config)
        .await
        .context("Failed to create app")?;

    println!("App `{}` created!", app.name);

    Ok(())
}

async fn get_app_credentials(context: &Context, name: &str) -> anyhow::Result<()> {
    context.assert_token();

    let app = show_short_app_info(context, name, false).await?;

    const CREDENTIALS_PATH: &str = "credentials.txt";

    tokio::fs::write(
        CREDENTIALS_PATH,
        format!(
            r#"# MAF service client credentials for {name}
MAF_CLIENT_ID={client_id}
MAF_CLIENT_SECRET={secret}"#,
            name = app.name,
            client_id = app.api_client_id,
            secret = app.api_secret
        ),
    )
    .await?;

    println!("Credentials for app `{name}` written to `{CREDENTIALS_PATH}`");

    Ok(())
}

async fn delete_app(context: &Context, name: String) -> anyhow::Result<()> {
    context.assert_token();

    show_short_app_info(context, &name, false).await?;

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
        .delete::<maf_schemas::apps::App>(format!("/api/v1/apps/{name}"), ())
        .await?;

    Ok(())
}
