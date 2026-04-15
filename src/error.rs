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
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
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

    #[error("Service Unavailable: {0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, error_code) = match &self {
            Self::Internal(err) => {
                error!("Internal error occurred: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    Some("INTERNAL_SERVER_ERROR".to_string()),
                )
            }
            Self::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                msg.clone(),
                Some("NOT_FOUND".to_string()),
            ),
            Self::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                msg.clone(),
                Some("VALIDATION_ERROR".to_string()),
            ),
            Self::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                msg.clone(),
                Some("UNAUTHORIZED".to_string()),
            ),
            Self::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                msg.clone(),
                Some("FORBIDDEN".to_string()),
            ),
            Self::Conflict(msg) => (
                StatusCode::CONFLICT,
                msg.clone(),
                Some("CONFLICT".to_string()),
            ),
            Self::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                msg.clone(),
                Some("SERVICE_UNAVAILABLE".to_string()),
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
