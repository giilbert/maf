use std::str::FromStr;

use http::{header::IntoHeaderName, uri::InvalidUri, HeaderMap, Uri};
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
#[derive(Debug)]
pub struct Request {
    uri: Uri,
    method: Method,
    headers: HeaderMap,
    body: RequestBody,
}

#[derive(Debug)]
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
    #[error("Invalid URI: {0}")]
    InvalidUri(#[from] InvalidUri),
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

pub trait IntoUri {
    fn into_uri(self) -> Result<Uri, InvalidUri>;
}

impl IntoUri for &'_ str {
    fn into_uri(self) -> Result<Uri, InvalidUri> {
        Uri::from_str(self)
    }
}

impl IntoUri for String {
    fn into_uri(self) -> Result<Uri, InvalidUri> {
        Uri::from_str(&self)
    }
}

impl IntoUri for Uri {
    fn into_uri(self) -> Result<Uri, InvalidUri> {
        Ok(self)
    }
}

impl Request {
    pub fn new(method: Method, url: impl IntoUri) -> RequestBuilder {
        RequestBuilder {
            wip: url
                .into_uri()
                .map(|url| Request {
                    uri: url.into(),
                    method,
                    headers: HeaderMap::new(),
                    body: RequestBody::None,
                })
                .map_err(RequestError::InvalidUri),
        }
    }

    pub async fn send(self) -> Result<Response, RequestError> {
        let req = OutgoingRequest::new(
            header_map_to_fields(&self.headers).map_err(RequestError::InvalidHeader)?,
        );

        req.set_method(&self.method)
            .map_err(|_| RequestError::InvalidMethod(self.method))?;

        let scheme_owned = Scheme::Other(
            self.uri
                .scheme_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        );
        req.set_scheme(match self.uri.scheme() {
            Some(scheme) => {
                if scheme == &http::uri::Scheme::HTTP {
                    Some(&Scheme::Http)
                } else if scheme == &http::uri::Scheme::HTTPS {
                    Some(&Scheme::Https)
                } else {
                    Some(&scheme_owned)
                }
            }
            None => None,
        })
        .map_err(|_| {
            RequestError::InvalidScheme(
                self.uri.scheme().map(|s| s.to_string()).unwrap_or_default(),
            )
        })?;
        // req.set_path_with_query(Some(&self.uri.path_and_query()))
        if let Some(path_and_query) = self.uri.path_and_query() {
            req.set_path_with_query(Some(path_and_query.as_str()))
                .map_err(|_| RequestError::InvalidPath(path_and_query.to_string()))?;
        }
        req.set_authority(self.uri.authority().map(|a| a.as_str()))
            .map_err(|_| {
                RequestError::InvalidAuthority(
                    self.uri
                        .authority()
                        .map(|a| a.to_string())
                        .unwrap_or_default(),
                )
            })?;

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
                let mut left = &data[..];

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

                    left = &left[write_amount..];
                }

                if !left.is_empty() {
                    return Err(RequestError::Body(StreamError::Closed));
                }

                drop(body_stream);
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
