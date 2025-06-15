use std::path::PathBuf;

use anyhow::Context;
use schemas::apps::RoomCreationStrategy;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ProjectConfig {
    pub data: ProjectConfigFile,
    pub base_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfigFile {
    pub name: String,
    #[serde(default = "default_room_creation_strategy")]
    pub rooms: RoomCreationStrategy,

    pub debug: TargetConfig,
    pub release: TargetConfig,
}

fn default_room_creation_strategy() -> RoomCreationStrategy {
    RoomCreationStrategy::AuthenticatedApiRequest
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetConfig {
    pub command: String,
    pub output: String,
}

impl ProjectConfig {
    pub async fn load() -> anyhow::Result<Option<Self>> {
        let mut current_directory = std::env::current_dir()?;

        loop {
            let config_path = current_directory.join("maf-project.toml");

            if tokio::fs::try_exists(&config_path).await.ok() == Some(true) {
                let content = tokio::fs::read_to_string(&config_path).await?;

                return Ok(Some(ProjectConfig {
                    base_path: current_directory.clone(),
                    data: toml::from_str(&content).context("Failed to parse maf-project.toml")?,
                }));
            }

            if !current_directory.pop() {
                break;
            }
        }

        Ok(None)
    }
}

pub fn handle_init_project() -> anyhow::Result<()> {
    Ok(())
}
