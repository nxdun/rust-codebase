use axum::http::HeaderMap;

use crate::config::AppConfig;

pub const API_KEY_HEADER: &str = "x-api-key";

fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
}

pub fn has_valid_master_api_key(headers: &HeaderMap, config: &AppConfig) -> bool {
    extract_api_key(headers).is_some_and(|api_key| api_key == config.master_api_key)
}
