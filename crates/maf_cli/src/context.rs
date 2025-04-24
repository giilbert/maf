use anyhow::Context as _;
use reqwest::header::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

use crate::pretty;

pub struct Context {
    pub client: reqwest::Client,
    server_url: Url,
}

impl Context {
    pub fn new() -> anyhow::Result<Self> {
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

        let server_url = Url::parse(&dotenvy::var("MAF_CLI_SERVER_URL").context(
            "failed to get server base url (MAF_CLI_SERVER_URL environment variable not set)",
        )?)
        .context("failed to parse server base url")?;

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build client")?;

        Ok(Context { client, server_url })
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
            .join(url.as_ref())
            .context("failed to join url")
    }

    pub async fn get<T: DeserializeOwned>(&self, url: impl AsRef<str>) -> anyhow::Result<T> {
        let response = self
            .client
            .get(self.url(url).context("failed to join url")?)
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(response.json().await?);
        } else {
            return Err(handle_error_response(response).await?);
        }
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        url: impl AsRef<str>,
        body: impl Serialize,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .post(self.url(url).context("failed to join url")?)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(response.json().await?);
        } else {
            return Err(handle_error_response(response).await?);
        }
    }

    pub async fn delete<T: DeserializeOwned>(
        &self,
        url: impl AsRef<str>,
        body: impl Serialize,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .delete(self.url(url).context("failed to join url")?)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            return Ok(response.json().await?);
        } else {
            return Err(handle_error_response(response).await?);
        }
    }
}

pub async fn handle_error_response(response: reqwest::Response) -> anyhow::Result<anyhow::Error> {
    let status = response.status();
    let response = response.json::<ErrorResponse>().await?;

    if response.r#type != "error" {
        anyhow::bail!("Unable to get error message from server");
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
