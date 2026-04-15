use crate::error::AppError;
use axum::{http::Uri, response::IntoResponse};
use tracing::warn;

/// Fallback handler for unknown routes.
pub async fn not_found_handler(uri: Uri) -> impl IntoResponse {
    warn!("404 Not Found: {}", uri.path());
    AppError::NotFound(format!("No route found for '{}'", uri.path())).into_response()
}
