use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context as _;
use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use futures_util::io::Cursor;
use futures_util::TryStreamExt;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use reqwest::Body;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::app::show_short_app_info;
use crate::dev::run_build_command;
use crate::{pretty, Context};

/// `maf app deploy <name> <wasm_module_path>` command handler
///
/// If both `name` and `wasm_module_path` are provided, deploys the specified path to the specified
/// app. If neither is provided, builds and deploys the current project's app.
pub async fn handle_deploy(
    context: &Context,
    name: Option<String>,
    wasm_module_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    context.assert_token();

    let (name, wasm_module_path) = match (name, wasm_module_path) {
        // The app is already built into a bundle at the specified path
        (Some(name), Some(path)) => (name, path),
        // The app needs to be built from the current project
        _ => {
            let project = context.assert_project();
            let name = project.data.name.clone();

            run_build_command(&project.base, &project.data.release.command)?;

            let output_path =
                tokio::fs::canonicalize(project.base.join(project.data.release.output.clone()))
                    .await
                    .context("Unable to find output file")?;

            (name, output_path)
        }
    };

    println!();

    // Check if the app exists
    let should_create_app = show_short_app_info(context, &name, true).await.is_err();

    if should_create_app {
        pretty::info!("This deployment will:");
        pretty::info!("- Create a new app named `{}`.", name.bold(),);
        pretty::info!(
            "- Deploy the bundle located at `{}`.",
            wasm_module_path.display()
        );
    } else {
        pretty::info!("This deployment will:");
        pretty::info!(
            "- Deploy the bundle located at `{}` to existing app {}.",
            wasm_module_path.display(),
            name.bold()
        );
        pretty::warn!("- This deployment will overwrite the existing deployment.",);
    }

    let confirmed = dialoguer::Confirm::new()
        .with_prompt("Do you want to continue?")
        .default(false)
        .interact()?;
    if !confirmed {
        pretty::error!("Aborted");
        return Ok(());
    }

    if should_create_app {
        pretty::info!("Creating app `{}`...", name.bold());
        crate::app::create_app(context).await?;
    }

    pretty::info!("Deploying app `{}`...\n", name.bold());

    deploy_wasm_module(context, name, &wasm_module_path).await
}

/// Deploys a WASM module located at `path` to the app named `name`.
async fn deploy_wasm_module(context: &Context, name: String, path: &Path) -> anyhow::Result<()> {
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

    println!();

    // Update the app's config
    if let Some(config) = &context.project_config {
        println!();
        pretty::info!(
            "Updating app configuration to {}...",
            config.base.join("maf-project.toml").display()
        );

        let config_string =
            toml::to_string_pretty(&config.data).context("failed to serialize project config")?;

        let config_update_response = context
            .client
            .post(context.url(format!("/api/v1/apps/{name}"))?)
            .json(&serde_json::json!({
                "config": config_string,
            }))
            .send()
            .await?;

        if !config_update_response.status().is_success() {
            anyhow::bail!(crate::context::handle_error_response(config_update_response).await?);
        }
    }

    pretty::info!("Deployment successful!");

    Ok(())
}

/// Creates a zip bundle containing the provided file as `module.wasm`.
///
/// Returns the zip bundle data and the original file's metadata.
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

    const ZIP_BUFFER_SIZE: usize = 50 * 1024 * 1024; // 50 MB

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
