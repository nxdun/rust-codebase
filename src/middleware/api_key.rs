use axum::http::HeaderMap;

use crate::config::AppConfig;

pub const API_KEY_HEADER: &str = "x-api-key";

/// Checks if the request headers contain a valid master API key.
#[must_use]
pub fn has_valid_master_api_key(headers: &HeaderMap, config: &AppConfig) -> bool {
    headers
        .get(API_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == config.master_api_key)
}
