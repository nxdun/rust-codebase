use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::{config::AppConfig, error::AppError, middleware::HEADER_API_KEY, state::AppState};

/// validation.inexpensive: Checks if the request headers contain a valid master API key.
#[must_use]
pub fn has_valid_master_api_key(headers: &HeaderMap, config: &AppConfig) -> bool {
    headers
        .get(HEADER_API_KEY)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| config.check_api_key(v))
}

/// Middleware that requires a valid master API key to be present in the headers.
pub async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if has_valid_master_api_key(req.headers(), state.config.as_ref()) {
        Ok(next.run(req).await)
    } else {
        Err(AppError::Unauthorized(
            "Invalid or missing API key".to_string(),
        ))
    }
}
