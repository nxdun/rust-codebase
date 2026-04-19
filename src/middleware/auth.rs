use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, middleware::api_key::API_KEY_HEADER, state::AppState};

/// Performs a constant-time comparison of two strings to prevent timing attacks.
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Middleware that requires a valid master API key to be present in the headers.
pub async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let api_key = req
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok());

    match api_key {
        Some(key) if constant_time_eq(key, &state.config.master_api_key) => Ok(next.run(req).await),
        _ => Err(AppError::Unauthorized(
            "Invalid or missing API key".to_string(),
        )),
    }
}
