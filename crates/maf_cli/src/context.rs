use anyhow::Context as _;
use reqwest::header::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

use crate::{
    config::{GlobalConfig, ProjectConfig},
    pretty,
};

pub struct Context {
    pub client: reqwest::Client,
    pub global_config: GlobalConfig,
    pub project_config: Option<ProjectConfig>,

    server_url: Option<Url>,
}

impl Context {
    pub fn new() -> anyhow::Result<Self> {
        let global_config = GlobalConfig::load()?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", global_config.token.as_deref().unwrap_or(""))
                .parse()
                .context("failed to parse header")?,
        );

        let server_url = global_config
            .server_url
            .clone()
            .and_then(|url| Url::parse(&url).ok());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build client")?;

        Ok(Context {
            client,
            server_url,
            global_config,
            project_config: ProjectConfig::load()?,
        })
    }

    pub fn assert_project(&self) -> &ProjectConfig {
        match &self.project_config {
            Some(config) => config,
            None => {
                pretty::error!(
                    "No project configuration found in the current directory or any parent directory."
                );
                std::process::exit(1);
            }
        }
    }

    pub fn assert_token(&self) {
        let token = self.global_config.token.as_deref();
        if token.is_some_and(|t| !t.is_empty()) {
        } else {
            pretty::error!(
                "You need to be logged in to perform this action. Please run `maf auth login`."
            );
            std::process::exit(1);
        }
    }

    pub fn url(&self, url: impl AsRef<str>) -> anyhow::Result<Url> {
        self.server_url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("MAF_CLI_SERVER_URL environment variable is not set"))?
            .join(url.as_ref())
            .context("failed to join url")
    }

    pub async fn fetch(
        &self,
        url: impl AsRef<str>,
        fetch: impl FnOnce(reqwest::Client, Url) -> reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::Response> {
        let url = self.url(url).context("failed to join url")?;
        let request = fetch(self.client.clone(), url);
        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response)
        } else {
            Err(handle_error_response(response).await?)
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, url: impl AsRef<str>) -> anyhow::Result<T> {
        self.fetch(url, |client, url| client.get(url))
            .await?
            .json()
            .await
            .context("Failed to deserialize response")
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        url: impl AsRef<str>,
        body: impl Serialize,
    ) -> anyhow::Result<T> {
        self.fetch(url, |client, url| client.post(url).json(&body))
            .await?
            .json()
            .await
            .context("Failed to deserialize response")
    }

    pub async fn delete<T: DeserializeOwned>(
        &self,
        url: impl AsRef<str>,
        body: impl Serialize,
    ) -> anyhow::Result<T> {
        self.fetch(url, |client, url| client.delete(url).json(&body))
            .await?
            .json()
            .await
            .context("Failed to deserialize response")
    }
}

pub async fn handle_error_response(response: reqwest::Response) -> anyhow::Result<anyhow::Error> {
    let status = response.status();
    let response = response.json::<ErrorResponse>().await?;

    if response.r#type != "error" {
        anyhow::bail!("Unable to get error message from server.");
    }

    Err(anyhow::anyhow!(
        "Request failed with status: {}.\nMessage: \"{}\"",
        status,
        response.data.message
    ))
}

#[derive(Deserialize)]
struct ErrorResponse {
    #[serde(rename = "type")]
    r#type: String,
    data: ErrorMessage,
}

#[derive(Deserialize)]
struct ErrorMessage {
    message: String,
}
