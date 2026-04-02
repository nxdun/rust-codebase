use axum::http::HeaderMap;

use crate::config::AppConfig;

pub const API_KEY_HEADER: &str = "x-api-key";

// Safe: Constant time comparison.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

//helper: to extract API key from headers
pub fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
}

//Validate with master API key environment variable
pub fn has_valid_master_api_key(headers: &HeaderMap, config: &AppConfig) -> bool {
    extract_api_key(headers)
        .is_some_and(|api_key| constant_time_eq(api_key, &config.master_api_key))
}
