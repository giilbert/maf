use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub server_url: Option<String>,
    pub token: Option<String>,
}

impl GlobalConfig {
    pub fn config_directory() -> anyhow::Result<PathBuf> {
        let directory = if cfg!(target_os = "windows") {
            PathBuf::from(std::env::var("APPDATA").context("Missing APPDATA environment variable")?)
                .join(".maf")
        } else if cfg!(target_os = "macos") {
            PathBuf::from("~/Library/Preferences").join(".maf")
        } else if cfg!(target_os = "linux") {
            PathBuf::from(std::env::var("HOME").context("Missing HOME environment variable")?)
                .join(".maf")
        } else {
            return Err(anyhow::anyhow!("Unsupported operating system"));
        };

        if !directory.exists() {
            std::fs::create_dir_all(&directory)
                .with_context(|| format!("Failed to create config directory at {:?}", directory))?;
        }

        Ok(directory)
    }

    pub fn get_config_file() -> anyhow::Result<PathBuf> {
        let config_dir = Self::config_directory()?;
        Ok(config_dir.join("config.toml"))
    }

    pub async fn load() -> anyhow::Result<Self> {
        if !fs::try_exists(Self::get_config_file()?).await? {
            return Ok(Self::default());
        }

        let mut config_data = toml::from_str::<GlobalConfig>(
            &fs::read_to_string(Self::get_config_file()?)
                .await
                .context("Failed to read config file")?,
        )
        .context("Failed to parse config file")?;

        if let Some(ref url) = config_data.server_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(anyhow::anyhow!(
                    "Server URL must start with 'http://' or 'https://'"
                ));
            }
        }

        if let Some(token_variable) = dotenvy::var("MAF_CLI_TOKEN").ok() {
            config_data.token = Some(token_variable);
        }

        if let Some(server_url_variable) = dotenvy::var("MAF_CLI_SERVER_URL").ok() {
            config_data.server_url = Some(server_url_variable);
        }

        Ok(config_data)
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let config_file = Self::get_config_file()?;
        let config_data = toml::to_string(self).context("Failed to serialize config data")?;

        fs::write(config_file, config_data)
            .await
            .context("Failed to write config file")?;

        Ok(())
    }
}
