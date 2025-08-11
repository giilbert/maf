use http::{header::IntoHeaderName, HeaderMap};
use url::{ParseError, Url};
use wasi::{
    http::{
        outgoing_handler::RequestOptions,
        types::{ErrorCode, HeaderError, Method, OutgoingBody, OutgoingRequest, Scheme},
    },
    io::streams::StreamError,
};

use crate::{
    http::{header_map_to_fields, response::Response},
    tasks,
};

/// Represents an HTTP request that hasn't been sent yet.
pub struct Request {
    url: Url,
    method: Method,
    headers: HeaderMap,
    body: RequestBody,
}

pub enum RequestBody {
    /// No body.
    None,
    /// Body where the entire content is provided at once.
    Full(Vec<u8>),
}

pub struct RequestBuilder {
    wip: Result<Request, RequestError>,
}

#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] ParseError),
    #[error("Invalid method: {0:?}")]
    InvalidMethod(Method),
    #[error("Invalid URL scheme: {0}")]
    InvalidScheme(String),
    #[error("Invalid URL path: {0}")]
    InvalidPath(String),
    #[error("Invalid URL authority: {0}")]
    InvalidAuthority(String),
    #[error("Invalid header: {0}")]
    InvalidHeader(HeaderError),
    #[error("Request failed. Error code: {0}")]
    RequestFailed(ErrorCode),
    #[error("Body write failed: {0}")]
    Body(StreamError),
}

pub trait IntoUrl {
    fn into_url(self) -> Result<Url, ParseError>;
}

impl IntoUrl for &'_ str {
    fn into_url(self) -> Result<Url, ParseError> {
        Url::parse(self)
    }
}

impl IntoUrl for String {
    fn into_url(self) -> Result<Url, ParseError> {
        Url::parse(&self)
    }
}

impl IntoUrl for Url {
    fn into_url(self) -> Result<Url, ParseError> {
        Ok(self)
    }
}

impl Request {
    pub fn new(url: impl IntoUrl) -> RequestBuilder {
        RequestBuilder {
            wip: url
                .into_url()
                .map(|url| Request {
                    url,
                    method: Method::Get,
                    headers: HeaderMap::new(),
                    body: RequestBody::None,
                })
                .map_err(RequestError::InvalidUrl),
        }
    }

    pub async fn send(self) -> Result<Response, RequestError> {
        let req = OutgoingRequest::new(
            header_map_to_fields(&self.headers).map_err(RequestError::InvalidHeader)?,
        );

        req.set_method(&self.method)
            .map_err(|_| RequestError::InvalidMethod(self.method))?;
        req.set_scheme(Some(&match self.url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            other => Scheme::Other(other.to_string()),
        }))
        .map_err(|_| RequestError::InvalidScheme(self.url.scheme().to_string()))?;
        req.set_path_with_query(Some(&format!(
            "{}{}{}{}",
            self.url.host_str().unwrap_or_default(),
            self.url
                .port()
                .map(|port| format!(":{port}"))
                .unwrap_or_default(),
            self.url.path(),
            self.url
                .query()
                .map(|q| format!("?{}", q))
                .unwrap_or_default()
        )))
        .map_err(|_| RequestError::InvalidPath(self.url.path().to_string()))?;
        req.set_authority(if self.url.has_authority() {
            Some(self.url.authority())
        } else {
            None
        })
        .map_err(|_| RequestError::InvalidAuthority(self.url.authority().to_string()))?;

        let options = RequestOptions::new();

        let body = req.body().expect("Body should be available");
        // Start the request with the provided options, sending the body separately.
        let future_response = wasi::http::outgoing_handler::handle(req, Some(options))
            .map_err(RequestError::RequestFailed)?;

        // Send request body
        match self.body {
            RequestBody::None => {}
            RequestBody::Full(data) => {
                let body_stream = body.write().expect("Body should be writable");
                let left = &data[..];

                loop {
                    tasks::wait_for(body_stream.subscribe()).await;

                    let permit_write =
                        body_stream.check_write().map_err(RequestError::Body)? as usize;
                    let write_amount = left.len().min(permit_write);

                    if write_amount == 0 {
                        break;
                    }

                    body_stream
                        .write(&left[..write_amount])
                        .map_err(RequestError::Body)?;
                }

                if !left.is_empty() {
                    return Err(RequestError::Body(StreamError::Closed));
                }

                OutgoingBody::finish(body, None).map_err(RequestError::RequestFailed)?;
            }
        }

        tasks::wait_for(future_response.subscribe()).await;

        let response = future_response
            .get()
            .expect("Response should be available")
            .expect("Response should not have been taken");

        let response = response.map_err(RequestError::RequestFailed)?;
        Ok(Response::from(response))
    }
}

impl RequestBuilder {
    /// Sets the method of the request.
    pub fn method(mut self, method: Method) -> Self {
        self.wip.as_mut().map(|req| req.method = method).ok();
        self
    }

    /// Sets the raw body of the request.
    pub fn body(mut self, body: RequestBody) -> Self {
        match &body {
            RequestBody::None => (),
            RequestBody::Full(bytes) => {
                self = self.header(http::header::CONTENT_LENGTH, bytes.len().to_string());
            }
        };
        self.wip.as_mut().map(|req| req.body = body).ok();
        self
    }

    /// Sets the body of the request as a JSON object.
    ///
    /// This will serialize the provided object into JSON, set the `Content-Type`` header to
    /// `application/json`, and the `Content-Length` header to the length of the serialized JSON.
    ///
    /// If the serialization fails, the builder will return an error once [`RequestBuilder::build`]
    /// is called.
    pub fn json(mut self, json: impl serde::Serialize) -> Self {
        match serde_json::to_vec(&json) {
            Ok(data) => self
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(RequestBody::Full(data)),
            Err(_) => {
                self.wip = Err(RequestError::InvalidHeader(HeaderError::InvalidSyntax));
                self
            }
        }
    }

    /// Sets the body of the request as plain text.
    ///
    /// This will set the `Content-Type` header to `text/plain` and the `Content-Length` header to
    /// the length of the text.
    pub fn text(self, text: impl AsRef<str>) -> Self {
        self.header(http::header::CONTENT_TYPE, "text/plain")
            .body(RequestBody::Full(text.as_ref().as_bytes().to_vec()))
    }

    /// Sets a single header by name and value.
    ///
    /// If the header value is invalid, the builder will return an error once
    /// [`RequestBuilder::build`] is called.
    pub fn header(mut self, key: impl IntoHeaderName, value: impl AsRef<str>) -> Self {
        match &mut self.wip {
            Ok(req) => {
                req.headers.insert(
                    key,
                    match value.as_ref().parse() {
                        Ok(v) => v,
                        Err(_) => {
                            self.wip = Err(RequestError::InvalidHeader(HeaderError::InvalidSyntax));
                            return self;
                        }
                    },
                );
            }
            Err(_) => {}
        }

        self
    }

    /// Sets multiple headers at once through a [`HeaderMap`].
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.wip.as_mut().map(|req| req.headers = headers).ok();
        self
    }

    #[must_use = "RequestBuilder::build() does not send the request"]
    pub fn build(self) -> Result<Request, RequestError> {
        self.wip
    }

    #[must_use]
    pub fn send(self) -> impl std::future::Future<Output = Result<Response, RequestError>> {
        async move { self.build()?.send().await }
    }
}
