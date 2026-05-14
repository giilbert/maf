use std::{fs, path::PathBuf};

use anyhow::Context;
use maf_schemas::{apps::RoomCreationStrategy, project_config::ProjectConfigFile};

/// Configuration information for a MAF project, if found in the current directory or any parent
/// directory.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub data: ProjectConfigFile,
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

/// Utility trait to working with [`ProjectConfig`] and [`Option<ProjectConfig>`].
pub trait ProjectConfigExt {
    fn room_creation_strategy_or_default(&self) -> RoomCreationStrategy;
}

impl ProjectConfigExt for ProjectConfig {
    fn room_creation_strategy_or_default(&self) -> RoomCreationStrategy {
        self.data.rooms
    }
}

impl ProjectConfigExt for Option<ProjectConfig> {
    fn room_creation_strategy_or_default(&self) -> RoomCreationStrategy {
        self.as_ref()
            .map(|config| config.room_creation_strategy_or_default())
            .unwrap_or(RoomCreationStrategy::AutoCreate)
    }
}
