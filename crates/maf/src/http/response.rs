use http::header::AsHeaderName;
use http::{HeaderMap, HeaderValue, StatusCode};
use wasi::http::types::{IncomingBody, IncomingResponse};
use wasi::io::streams::{InputStream, StreamError};

use crate::tasks;

pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: ResponseBody,
}

impl From<IncomingResponse> for Response {
    fn from(value: IncomingResponse) -> Self {
        Response {
            status: StatusCode::from_u16(value.status()).expect("Invalid status code"),
            headers: crate::http::fields_to_header_map(&value.headers())
                .expect("Failed to convert fields to header map"),
            body: ResponseBody {
                inner: value.consume().expect("Response body already consumed"),
            },
        }
    }
}

pub struct ResponseBody {
    inner: IncomingBody,
}

#[derive(Debug, thiserror::Error)]
pub enum ResponseBodyError {
    #[error("Failed to read response body: {0}")]
    ReadError(#[from] wasi::io::streams::StreamError),
    #[error("Response body already consumed")]
    AlreadyConsumed,
    #[error("Response body is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

impl ResponseBody {
    pub fn get(&self) -> Result<InputStream, ResponseBodyError> {
        self.inner
            .stream()
            .map_err(|_| ResponseBodyError::AlreadyConsumed)
    }
}

impl Response {
    /// Returns the HTTP status code of the response.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the headers of the response.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns the value of a specific header by name.
    pub fn header(&self, name: impl AsHeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Reads the entire response body as a vector of bytes.
    ///
    /// **A body can only be consumed once**—subsequent calls that attempt to read the body will
    /// return [`ResponseBodyError::AlreadyConsumed`].
    ///
    /// ## Errors
    /// Returns [`ResponseBodyError::AlreadyConsumed`] if the body has already been consumed,
    /// [`ResponseBodyError::ReadError`] if there was an error reading the body.
    pub async fn bytes(&self) -> Result<Vec<u8>, ResponseBodyError> {
        let stream = self.body.get()?;
        let mut buffer = Vec::new();

        loop {
            tasks::wait_for(stream.subscribe())
                .named("get response body")
                .await;

            match stream.read(u64::MAX) {
                Ok(data) => buffer.extend_from_slice(&data),
                Err(StreamError::Closed) => break,
                Err(e) => return Err(ResponseBodyError::ReadError(e)),
            }
        }

        Ok(buffer)
    }

    /// Reads the entire response body as a UTF-8 string.
    ///
    /// ## Errors
    /// Returns [`ResponseBodyError::InvalidUtf8`] if the body is not valid UTF-8 or other errors
    /// related to reading the body described by [`Self::bytes`].
    pub async fn text(&self) -> Result<String, ResponseBodyError> {
        self.bytes()
            .await
            .and_then(|bytes| String::from_utf8(bytes).map_err(ResponseBodyError::InvalidUtf8))
    }

    /// Reads the entire response body as JSON.
    ///
    /// ## Errors
    /// Returns [`ResponseBodyError::InvalidJson`] if the body is not valid JSON or other errors
    /// related to reading the body described by [`Self::text`].
    pub async fn json(&self) -> Result<serde_json::Value, ResponseBodyError> {
        let text = self.text().await?;
        serde_json::from_str(&text).map_err(ResponseBodyError::InvalidJson)
    }
}
