use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::Context;
use dialoguer::Select;
use include_dir::{include_dir, Dir, DirEntry};

use crate::input::input;
use crate::pretty;

static TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/init/templates");

#[derive(Debug, Clone, clap::Args)]
pub struct InitOptions {
    /// The name of the project to create. If not provided, you will be prompted to enter one.
    project_name: Option<String>,
    #[arg(short, long)]
    /// The template to use for the project. If not provided, you will be prompted to select one.
    template: Option<String>,
    /// The path to the directory where the project will be created. Defaults to the current directory.
    #[arg(short, long, default_value = ".")]
    path: PathBuf,
}

fn transform_project_name(name: &str) -> anyhow::Result<String> {
    if name.is_empty() {
        anyhow::bail!("Name cannot be empty.")
    }
    if name.len() > 100 {
        anyhow::bail!("Name cannot be longer than 100 characters.")
    }
    if !name.chars().all(|c| {
        (c.is_ascii_alphanumeric() && (c.is_ascii_lowercase() || c.is_numeric())) || c == '-'
    }) {
        anyhow::bail!("Name can only contain lowercase alphanumeric characters and hyphens.")
    }

    Ok(name.to_string())
}

pub fn handle_init(mut options: InitOptions) -> anyhow::Result<()> {
    match options.project_name.clone() {
        Some(name) => {
            transform_project_name(&name)?; // Validate the provided project name
            pretty::info!("Creating new project: {}", name.bold());
            run_setup_commands(options)
        }
        None => {
            let name = input!(
                transform: |name: String| {
                    transform_project_name(&name)
                },
                "{} {}:",
                "Name".bold(),
                "(Lowercase alphanumeric characters and hyphens)".dimmed()
            );

            options.project_name = Some(name.clone());
            run_setup_commands(options)
        }
    }
}

pub fn handle_create(mut options: InitOptions) -> anyhow::Result<()> {
    let server_path = input!(
        transform: |path: String| {
            if path.is_empty() {
                Ok("server".to_string())
            } else {
                Ok(path)
            }
        },
        "{} {}:",
        "Server path".bold(),
        "(Defaults to './server')".dimmed()
    );

    options.path = PathBuf::from(server_path);

    handle_init(options)?;

    Ok(())
}

fn run_setup_commands(options: InitOptions) -> anyhow::Result<()> {
    let project_name = options
        .project_name
        .expect("Project name should be set by now");

    let (template, template_name) = match options.template {
        Some(template) => (
            TEMPLATES
                .get_dir(&template)
                .ok_or_else(|| anyhow::anyhow!("Template '{}' not found.", template))?,
            template.clone(),
        ),
        None => {
            let template_names = TEMPLATES
                .dirs()
                .map(|entry: &Dir| {
                    entry
                        .path()
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                })
                .collect::<Vec<_>>();

            let selected_index = Select::new()
                .items(&template_names)
                .with_prompt(format!(
                    "{} {}",
                    "?".bold().purple(),
                    "Select a template".bold()
                ))
                .default(0)
                .interact()
                .map_err(|e| anyhow::anyhow!("Failed to select template: {}", e))?;

            let template_name = template_names
                .get(selected_index)
                .ok_or_else(|| anyhow::anyhow!("Invalid template selection."))?;

            (
                TEMPLATES
                    .get_dir(PathBuf::from(template_name.to_string()))
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Template '{}' not found in the included templates.",
                            template_name
                        )
                    })?,
                template_name.to_string(),
            )
        }
    };

    let confirmation = dialoguer::Confirm::new()
        .with_prompt(format!(
            "{} This will create a new project in '{}'. Do you want to continue?",
            "?".bold().purple(),
            options.path.display().to_string().bold()
        ))
        .default(false)
        .interact()
        .map_err(|e| anyhow::anyhow!("Failed to confirm project creation: {}", e))?;

    if !confirmation {
        pretty::info!("Project creation cancelled.");
        return Ok(());
    }

    println!();

    pretty::info!(
        "Setting up project '{}' using template '{}' in {}",
        project_name.bold(),
        template.path().display().to_string().bold(),
        options.path.display().to_string().bold()
    );

    println!();

    // Check if the project directory contains any files that would be overwritten
    fn check_entry_recurse(prefix: &str, base_path: &Path, entry: &DirEntry) -> anyhow::Result<()> {
        match entry {
            DirEntry::Dir(dir) => {
                for sub_entry in dir.entries() {
                    check_entry_recurse(prefix, base_path, sub_entry)?;
                }
            }
            DirEntry::File(file) => {
                let mut path = file.path().strip_prefix(prefix)?.to_path_buf();
                // Add the project path to the file path
                path = base_path.join(path);

                if std::fs::exists(&path)
                    .map_err(|e| anyhow::anyhow!("Failed to check if path exists: {}", e))?
                {
                    pretty::error!(
                        "File {} already exists. Please remove it before proceeding.",
                        path.display().to_string().bold(),
                    );

                    std::process::exit(1);
                }
            }
        }

        Ok(())
    }

    check_entry_recurse(
        &template_name,
        &options.path,
        &DirEntry::Dir(template.clone()),
    )
    .context("Failed to check for existing files")?;

    // Replaces {{<name>}} placeholders in the template files
    let mut templates = HashMap::new();
    templates.insert("name", project_name.clone());
    // Rust crate names cannot contain hyphens, so they need to be replaced with underscores
    templates.insert("crate_name", project_name.replace('-', "_"));

    fn extract_template_recurse(
        prefix: &str,
        base_path: &Path,
        templates: &HashMap<String, String>,
        entry: &DirEntry,
    ) -> anyhow::Result<()> {
        match entry {
            DirEntry::Dir(dir) => {
                let dir_path = base_path.join(dir.path().strip_prefix(prefix)?);

                std::fs::create_dir_all(dir_path).context(format!(
                    "Failed to create directory '{}'",
                    dir.path().display()
                ))?;

                for sub_entry in dir.entries() {
                    extract_template_recurse(prefix, base_path, templates, sub_entry)?;
                }
            }
            DirEntry::File(file) => {
                let file_path = base_path
                    .join(file.path().strip_prefix(prefix)?)
                    .to_string_lossy()
                    // Remove any `$` characters from the path, which are used to prevent
                    // language servers from trying to resolve the file where they're not needed
                    .replace("$", "");

                let content = file
                    .contents_utf8()
                    .context(format!("Failed to read file '{}'", file_path))?;

                // Replace placeholders in the content
                let content = templates
                    .iter()
                    .fold(content.to_string(), |acc, (key, value)| {
                        acc.replace(key, value)
                    });

                std::fs::write(&file_path, content)
                    .context(format!("Failed to write file '{}'", file_path))?;
            }
        }

        Ok(())
    }

    extract_template_recurse(
        &template_name,
        &options.path,
        &templates
            .into_iter()
            .map(|(k, v)| (format!("{{{{{}}}}}", k), v))
            .collect(),
        &DirEntry::Dir(template.clone()),
    )
    .context("Failed to extract template files")?;

    // Perform additional setup based on the selected template
    // e.g. installing tools and dependencies
    if template_name.as_str() == "rust" {
        additional_rust_setup()?
    }

    pretty::info!(
        "Done! Your project '{}' has been initialized using the '{}' template.",
        project_name.bold(),
        template_name.bold()
    );

    Ok(())
}

fn additional_rust_setup() -> anyhow::Result<()> {
    pretty::info!("Installing `wasm32-wasip2` target for Rust...");

    println!("{} rustup target add wasm32-wasip2", "$".bold().purple());
    let status = process::Command::new("rustup")
        .args(["target", "add", "wasm32-wasip2"])
        .status()?;

    if status.code().unwrap_or(1) != 0 {
        pretty::warn!("Failed to install `wasm32-wasip2` target. Please install it manually.");
    }

    println!();

    Ok(())
}
