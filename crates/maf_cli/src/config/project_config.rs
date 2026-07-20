use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use maf_schemas::project_config::ProjectConfigFile;

/// Configuration information for a MAF project, if found in the current directory or any parent
/// directory.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// The parsed `maf-project.toml` file for the project.
    pub data: ProjectConfigFile,
    /// The base path of the project, which is the directory that contains the `maf-project.toml`
    /// file.
    pub base: PathBuf,
}

impl ProjectConfig {
    pub fn load() -> anyhow::Result<Option<Self>> {
        let mut current_directory = std::env::current_dir()?;

        loop {
            let config_path = current_directory.join("maf-project.toml");
            if fs::exists(&config_path).is_ok_and(|exists| exists) {
                let content = fs::read_to_string(&config_path)?;

                let data: ProjectConfigFile =
                    toml::from_str(&content).context("Failed to parse maf-project.toml")?;

                data.validate()
                    .map_err(|e| anyhow::anyhow!("Error validating: maf-project.toml: {}", e))?;

                return Ok(Some(ProjectConfig {
                    base: current_directory.clone(),
                    data,
                }));
            }

            // Continue to the parent directory. If there is no parent directory, stop.
            if !current_directory.pop() {
                break;
            }
        }

        Ok(None)
    }
}
