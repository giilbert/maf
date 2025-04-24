mod models;

use std::{iter::zip, time::Duration};

use anyhow::Context as _;
use async_zip::{tokio::write::ZipFileWriter, Compression, ZipEntryBuilder, ZipFileBuilder};
use clap::Subcommand;
use colored::Colorize;
use futures_util::{io::Cursor, StreamExt, TryStreamExt};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use reqwest::Body;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_util::{
    codec::{BytesCodec, FramedRead},
    compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt, TokioAsyncWriteCompatExt},
};
use uuid::Uuid;

use crate::{input::input, pretty, Context};

#[derive(Subcommand, Debug, Clone)]
pub enum AppCommands {
    List,
    Create,
    Delete { name: String },
    Deploy { name: String, path: String },
}

pub async fn handle_commands(context: &Context, command: AppCommands) -> anyhow::Result<()> {
    match command {
        AppCommands::List => list_apps(context).await,
        AppCommands::Create => create_app(context).await,
        AppCommands::Delete { name } => delete_app(context, name).await,
        AppCommands::Deploy { name, path } => deploy_bundle(context, name, path).await,
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

async fn deploy_bundle(context: &Context, name: String, path: String) -> anyhow::Result<()> {
    const MAX_SIZE: usize = 20 * 1024 * 1024; // 20 MB

    let mut file = tokio::fs::File::open(&path)
        .await
        .with_context(|| format!("failed to open file `{path}`"))?;

    let metadata = file
        .metadata()
        .await
        .with_context(|| format!("failed to get metadata for file `{path}`"))?;

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

    let zip_bundle_bar = ProgressBar::new_spinner();
    zip_bundle_bar.enable_steady_tick(Duration::from_millis(100));
    zip_bundle_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.magenta} {wide_msg} [{elapsed_precise}]")?,
    );
    zip_bundle_bar.set_message("Creating zip bundle...");

    const ZIP_BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8 MB

    let zip_buffer = tokio::io::BufWriter::with_capacity(ZIP_BUFFER_SIZE, Vec::new());
    let mut zip = ZipFileWriter::new(zip_buffer.compat_write());

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

    let compressed_data = zip_writer.into_inner();
    let bar = ProgressBar::new(compressed_data.len() as u64);
    let bar_clone = bar.clone();

    let stream = FramedRead::new(Cursor::new(compressed_data).compat(), BytesCodec::new())
        .inspect_ok(move |chunk| {
            bar_clone.update(|state| state.set_pos(state.pos() + chunk.len() as u64));
        });

    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_message(format!("Uploading `{}`\n", path));
    bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.magenta} {msg} {wide_bar} {bytes}/{total_bytes} [eta: {eta}] [{elapsed_precise}]",
    )?);

    let response = context
        .client
        .post(context.url(format!("/api/apps/{name}/deployments"))?)
        .body(Body::wrap_stream(stream))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(crate::context::handle_error_response(response).await?);
    }

    bar.set_style(ProgressStyle::default_bar().template("{wide_msg} [{elapsed_precise}]")?);
    bar.finish_with_message(format!(
        "Uploaded `{}` ({} bytes)",
        path,
        HumanBytes(metadata.len())
    ));

    println!("");

    Ok(())
}
