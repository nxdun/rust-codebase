use std::borrow::Cow;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    status: u16,
    message: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<Cow<'static, str>>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Internal Server Error")]
    Internal(#[from] anyhow::Error),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Upstream Service Error: {0}")]
    UpstreamError(String),

    #[error("Service Unavailable: {0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, error_code): (
            StatusCode,
            Cow<'static, str>,
            Option<Cow<'static, str>>,
        ) = match &self {
            Self::Internal(err) => {
                error!("Internal error occurred: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Cow::Borrowed("Internal Server Error"),
                    Some(Cow::Borrowed("INTERNAL_SERVER_ERROR")),
                )
            }
            Self::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                Cow::Owned(msg.clone()),
                Some(Cow::Borrowed("NOT_FOUND")),
            ),
            Self::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Cow::Owned(msg.clone()),
                Some(Cow::Borrowed("VALIDATION_ERROR")),
            ),
            Self::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                Cow::Owned(msg.clone()),
                Some(Cow::Borrowed("UNAUTHORIZED")),
            ),
            Self::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                Cow::Owned(msg.clone()),
                Some(Cow::Borrowed("FORBIDDEN")),
            ),
            Self::Conflict(msg) => (
                StatusCode::CONFLICT,
                Cow::Owned(msg.clone()),
                Some(Cow::Borrowed("CONFLICT")),
            ),
            Self::UpstreamError(msg) => {
                tracing::error!("Upstream service error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    Cow::Borrowed("Upstream service error"),
                    Some(Cow::Borrowed("UPSTREAM_ERROR")),
                )
            }
            Self::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Cow::Owned(msg.clone()),
                Some(Cow::Borrowed("SERVICE_UNAVAILABLE")),
            ),
        };

        let body = Json(ErrorResponse {
            status: status.as_u16(),
            message,
            error_code,
        });

        (status, body).into_response()
    }
}

// Convert common error types into AppError
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(anyhow::anyhow!("JSON error: {err}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(anyhow::anyhow!("IO error: {err}"))
    }
}
