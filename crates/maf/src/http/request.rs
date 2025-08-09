use url::{ParseError, Url};
use wasi::{
    http::{
        outgoing_handler::RequestOptions,
        types::{ErrorCode, Fields, Method, OutgoingBody, OutgoingRequest, Scheme},
    },
    io::streams::StreamError,
};

use crate::tasks;

/// Represents an HTTP request that hasn't been sent yet.
pub struct Request {
    url: Url,
    method: Method,
    headers: Fields,
    body: RequestBody,
}

pub enum RequestBody {
    None,
    Full(Vec<u8>),
}

pub struct RequestBuilder {
    wip: Request,
}

#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    #[error("Invalid method: {0:?}")]
    InvalidMethod(Method),
    #[error("Invalid URL scheme: {0}")]
    InvalidScheme(String),
    #[error("Invalid URL path: {0}")]
    InvalidPath(String),
    #[error("Invalid URL authority: {0}")]
    InvalidAuthority(String),
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
    pub fn get(url: impl IntoUrl) -> RequestBuilder {
        RequestBuilder {
            wip: Request {
                url: url.into_url().expect("Invalid URL"),
                method: Method::Get,
                headers: Fields::new(),
                body: RequestBody::None,
            },
        }
    }

    pub async fn send(self) -> Result<(), RequestError> {
        let req = OutgoingRequest::new(self.headers);

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

        let headers = response.headers();
        let status = response.status();
        let body = response.consume().expect("Response already consumed");

        println!("Response status: {}", status);
        println!("Response headers: {:?}", headers.entries());

        let stream = body.stream().expect("Body stream should be available");
        let mut buffer = Vec::new();

        loop {
            tasks::wait_for(stream.subscribe()).await;

            match stream.read(u64::MAX) {
                Ok(data) => buffer.extend_from_slice(&data),
                Err(StreamError::Closed) => break,
                Err(e) => return Err(RequestError::Body(e)),
            }
        }

        println!("Response body: {:?}", String::from_utf8_lossy(&buffer));

        Ok(())
    }
}

impl RequestBuilder {
    pub fn method(mut self, method: Method) -> Self {
        self.wip.method = method;
        self
    }

    pub fn body(mut self, body: RequestBody) -> Self {
        self.wip.body = body;
        self
    }

    pub fn build(self) -> Request {
        self.wip
    }

    pub fn send(self) -> impl std::future::Future<Output = Result<(), RequestError>> {
        async move { self.build().send().await }
    }
}
