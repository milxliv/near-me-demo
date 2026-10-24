use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Every failure the API can return. Messages are fixed strings: no request
/// content is ever echoed back or written anywhere.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ApiError {
    #[error("origin not allowed")]
    Forbidden,

    #[error("send application/json")]
    UnsupportedMediaType,

    #[error("too many requests, try again in a minute")]
    TooManyRequests,

    #[error("bad request")]
    BadRequest,

    #[error("name must be 1-40 letters")]
    InvalidName,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiError::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ApiError::BadRequest | ApiError::InvalidName => StatusCode::BAD_REQUEST,
        };
        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}
