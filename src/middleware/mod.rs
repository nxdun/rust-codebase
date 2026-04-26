use axum::http::header::HeaderName;

pub mod api_key;
pub mod auth;
pub mod captcha;
pub mod cors;
pub mod rate_limit;

pub const X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");
pub const X_CAPTCHA_TOKEN: HeaderName = HeaderName::from_static("x-captcha-token");

/// Performs a constant-time comparison of two strings to prevent timing attacks.
#[must_use]
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
