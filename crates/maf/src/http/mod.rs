mod request;
mod response;

use std::str::FromStr;

use http::{HeaderMap, HeaderName};
pub use request::*;
use wasi::http::types::Fields;

macro_rules! impl_http_method_builder {
    ($name:ident, $method:expr) => {
        #[allow(unused)]
        pub fn $name(url: impl IntoUrl) -> RequestBuilder {
            Request::new(url)
        }
    };
}

impl_http_method_builder!(get, Method::Get);
impl_http_method_builder!(post, Method::Post);
impl_http_method_builder!(put, Method::Put);
impl_http_method_builder!(delete, Method::Delete);
impl_http_method_builder!(patch, Method::Patch);
impl_http_method_builder!(head, Method::Head);
impl_http_method_builder!(options, Method::Options);

fn header_map_to_fields(headers: &HeaderMap) -> Result<Fields, wasi::http::types::HeaderError> {
    let fields = Fields::new();
    for (key, value) in headers.iter() {
        fields.set(key.as_str(), &[value.as_bytes().to_vec()])?;
    }
    Ok(fields)
}

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(#[from] http::header::InvalidHeaderName),
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
}

fn fields_to_header_map(fields: &Fields) -> Result<HeaderMap, ConvertError> {
    let mut headers = HeaderMap::new();
    for (key, value) in fields.entries() {
        headers.insert(
            HeaderName::from_str(&key).map_err(ConvertError::InvalidHeaderName)?,
            String::from_utf8_lossy(&value)
                .parse()
                .map_err(ConvertError::InvalidHeaderValue)?,
        );
    }
    Ok(headers)
}
