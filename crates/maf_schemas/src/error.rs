use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub struct ErrorResponse {
    status_code: StatusCode,
    message: String,
}

macro_rules! impl_error_status {
    ($name:ident, $status:ident, $message:expr) => {
        impl ErrorResponse {
            #[allow(dead_code)]
            pub fn $name(message: Option<&str>) -> Self {
                tracing::debug!(
                    "error: {} - {}",
                    StatusCode::$status,
                    message
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| $message.to_string())
                );
                ErrorResponse {
                    status_code: StatusCode::$status,
                    message: message
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| $message.to_string()),
                }
            }
        }
    };
}

impl_error_status!(unauthorized, UNAUTHORIZED, "Unauthorized");
impl_error_status!(forbidden, FORBIDDEN, "Forbidden");
impl_error_status!(not_found, NOT_FOUND, "Not Found");
impl_error_status!(bad_request, BAD_REQUEST, "Bad Request");
impl_error_status!(
    internal_server_error,
    INTERNAL_SERVER_ERROR,
    "Internal Server Error"
);
impl_error_status!(conflict, CONFLICT, "Conflict");
impl_error_status!(bad_gateway, BAD_GATEWAY, "Bad Gateway");

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        Response::builder()
            .status(self.status_code)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(Body::new(
                serde_json::to_string(&json!({
                    "type": "error",
                    "data": {
                        "message": self.message,
                    }
                }))
                .expect("failed to serialize error response"),
            ))
            .expect("failed to build error response")
    }
}

impl<E> From<E> for ErrorResponse
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        let error = value.into();
        tracing::error!("Error response: {:?}", error);
        Self::internal_server_error(None)
    }
}
