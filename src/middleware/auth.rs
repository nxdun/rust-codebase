use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, middleware::X_API_KEY, state::AppState};

/// Middleware that requires a valid master API key to be present in the headers.
pub async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let api_key = req
        .headers()
        .get(X_API_KEY)
        .and_then(|value| value.to_str().ok());

    match api_key {
        Some(key) if state.config.check_api_key(key) => Ok(next.run(req).await),
        _ => Err(AppError::Unauthorized(
            "Invalid or missing API key".to_string(),
        )),
    }
}
