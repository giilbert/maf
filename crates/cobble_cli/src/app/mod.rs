mod models;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context as _;
use async_zip::{tokio::write::ZipFileWriter, Compression, ZipEntryBuilder};
use clap::Subcommand;
use cobble_schemas::apps::CreateUserAppRequest;
use colored::Colorize;
use futures_util::{io::Cursor, TryStreamExt};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use reqwest::Body;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::{
    codec::{BytesCodec, FramedRead},
    compat::{FuturesAsyncReadCompatExt, TokioAsyncWriteCompatExt},
};

use crate::{dev::run_build_command, input::input, pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum AppCommands {
    /// List all apps in the current organization
    List,
    /// Create a new app
    Create,
    /// Get service account credentials for an app by name, or the current project's app if no name is provided
    Credentials { name: Option<String> },
    /// Show information about an app by name, or the current project's app if no name is provided
    View { name: Option<String> },
    /// Delete an app by name, or the current project's app if no name is provided
    Delete { name: Option<String> },
    /// Deploy an app by name and path, or the current project's app if no name or path is provided
    Deploy {
        #[clap(requires = "path")]
        name: Option<String>,
        path: Option<String>,
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
        AppCommands::Deploy { name, path } => match (name, path) {
            (Some(name), Some(path)) => deploy_bundle(context, name, &PathBuf::from(path)).await,
            _ => {
                let project = context.assert_project();
                let name = project.data.name.clone();

                run_build_command(&project.base, &project.data.release.command)?;

                let output_path =
                    tokio::fs::canonicalize(project.base.join(project.data.release.output.clone()))
                        .await
                        .context("Unable to find output file")?;
                deploy_bundle(context, name, &output_path).await
            }
        },
    }
}

async fn list_apps(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let apps = context.get::<Vec<models::App>>("/api/v1/apps").await?;

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

async fn show_short_app_info(context: &Context, name: &str) -> anyhow::Result<models::App> {
    context.assert_token();

    println!("Fetching app `{name}`...\n");

    let app = context
        .get::<models::App>(format!("/api/v1/apps/{name}"))
        .await
        .context("failed to get app")?;

    println!(
        "{} {}",
        app.name.bold(),
        format!("(id: {})", app.id).dimmed()
    );
    println!("");

    Ok(app)
}

async fn view_app(context: &Context, name: &str) -> anyhow::Result<()> {
    context.assert_token();

    let app = show_short_app_info(context, name).await?;

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
                return Ok(());
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
        .post::<models::App>("/api/v1/apps", &config)
        .await
        .context("Failed to create app")?;

    println!("App `{}` created!", app.name);

    Ok(())
}

async fn get_app_credentials(context: &Context, name: &str) -> anyhow::Result<()> {
    context.assert_token();

    let app = show_short_app_info(context, name).await?;

    const CREDENTIALS_PATH: &str = "credentials.txt";

    tokio::fs::write(
        CREDENTIALS_PATH,
        format!(
            r#"# Cobble service client credentials for {name}
COBBLE_CLIENT_ID={client_id}
COBBLE_CLIENT_SECRET={secret}"#,
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

    show_short_app_info(context, &name).await?;

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
        .delete::<models::App>(format!("/api/v1/apps/{name}"), ())
        .await?;

    Ok(())
}

async fn deploy_bundle(context: &Context, name: String, path: &Path) -> anyhow::Result<()> {
    context.assert_token();

    show_short_app_info(context, &name).await?;

    let mut file = tokio::fs::File::open(&path)
        .await
        .with_context(|| format!("failed to open file `{path:?}`"))?;

    let (compressed_data, metadata) = create_zip_bundle(&mut file).await?;
    let bar = ProgressBar::new(compressed_data.len() as u64);
    let bar_clone = bar.clone();

    let stream = FramedRead::new(Cursor::new(compressed_data).compat(), BytesCodec::new())
        .inspect_ok(move |chunk| {
            bar_clone.update(|state| state.set_pos(state.pos() + chunk.len() as u64));
        });

    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_message(format!("Uploading `{path:?}`\n"));
    bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.magenta} {msg} {wide_bar} {bytes}/{total_bytes} [eta: {eta}] [{elapsed_precise}]",
    )?);

    let response = context
        .client
        .post(context.url(format!("/api/v1/apps/{name}/deployments"))?)
        .body(Body::wrap_stream(stream))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(crate::context::handle_error_response(response).await?);
    }

    bar.set_style(ProgressStyle::default_bar().template("{wide_msg} [{elapsed_precise}]")?);
    bar.finish_with_message(format!(
        "Uploaded `{path:?}` ({} bytes)",
        HumanBytes(metadata.len())
    ));

    println!("");

    Ok(())
}

async fn create_zip_bundle(
    file: &mut tokio::fs::File,
) -> anyhow::Result<(Vec<u8>, std::fs::Metadata)> {
    let zip_bundle_bar = ProgressBar::new_spinner();
    zip_bundle_bar.enable_steady_tick(Duration::from_millis(100));
    zip_bundle_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.magenta} {wide_msg} [{elapsed_precise}]")?,
    );
    zip_bundle_bar.set_message("Creating zip bundle...");

    const ZIP_BUFFER_SIZE: usize = 20 * 1024 * 1024; // 20 MB

    let zip_buffer = tokio::io::BufWriter::with_capacity(ZIP_BUFFER_SIZE, Vec::new());
    let mut zip = ZipFileWriter::new(zip_buffer.compat_write());

    let metadata = file
        .metadata()
        .await
        .context("failed to get metadata for file")?;

    let mut file_data = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut file_data).await?;
    zip.write_entry_whole(
        ZipEntryBuilder::new("module.wasm".into(), Compression::Deflate),
        &file_data,
    )
    .await?;

    let mut zip_writer = zip.close().await?.into_inner();
    zip_writer.flush().await?;
    zip_writer.shutdown().await?;

    zip_bundle_bar
        .set_style(ProgressStyle::default_bar().template("{wide_msg} [{elapsed_precise}]")?);
    zip_bundle_bar.finish_with_message(format!(
        "Created zip bundle ({} bytes)",
        HumanBytes(metadata.len())
    ));

    Ok((zip_writer.into_inner(), metadata))
}
