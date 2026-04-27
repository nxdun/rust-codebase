use axum::http::HeaderMap;

use crate::{config::AppConfig, middleware::X_API_KEY};

/// Checks if the request headers contain a valid master API key.
#[must_use]
pub fn has_valid_master_api_key(headers: &HeaderMap, config: &AppConfig) -> bool {
    headers
        .get(X_API_KEY)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| config.check_api_key(v))
}
