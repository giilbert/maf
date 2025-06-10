use anyhow::Context as _;
use reqwest::header::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

use crate::{config::ConfigOptions, pretty};

pub struct Context {
    pub client: reqwest::Client,
    pub config: ConfigOptions,
    server_url: Option<Url>,
}

impl Context {
    pub async fn new() -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!(
                "Bearer {}",
                dotenvy::var("MAF_CLI_TOKEN").unwrap_or("".to_string())
            )
            .parse()
            .context("failed to parse header")?,
        );

        let server_url = match dotenvy::var("MAF_CLI_SERVER_URL") {
            Ok(url) => Some(Url::parse(&url).context("failed to parse server url")?),
            Err(_) => None,
        };

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build client")?;

        let config = ConfigOptions::load().await?;

        Ok(Context {
            client,
            server_url,
            config,
        })
    }

    pub fn assert_token(&self) {
        let token = dotenvy::var("MAF_CLI_TOKEN");
        if token.is_ok_and(|t| t.len() > 0) {
            return;
        } else {
            pretty::error!("MAF_CLI_TOKEN environment variable is not set");
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
