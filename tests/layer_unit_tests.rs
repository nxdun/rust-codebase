#![allow(unsafe_code)]
use axum::http::HeaderMap;
use nadzu::{
    config::AppConfig,
    middleware::{api_key::has_valid_master_api_key, rate_limit::is_production},
    models::health::Health,
};

fn test_config(env: &str) -> AppConfig {
    AppConfig::new(
        "nadzu-test".to_string(),
        env.to_string(),
        "127.0.0.1".to_string(),
        8080,
        None,
        "downloads".to_string(),
        "yt-dlp".to_string(),
        None,
        None,
        3,
        None,
        "master_key".to_string(),
        None,
        None,
        "https://api.github.com/graphql".to_string(),
    )
}

#[test]
#[allow(clippy::unwrap_used)]
fn has_valid_master_api_key_returns_true_for_matching_header() {
    let config = test_config("test");
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", "master_key".parse().unwrap());

    assert!(has_valid_master_api_key(&headers, &config));
}

#[test]
#[allow(clippy::unwrap_used)]
fn has_valid_master_api_key_returns_false_for_missing_or_wrong_header() {
    let config = test_config("test");

    let empty_headers = HeaderMap::new();
    assert!(!has_valid_master_api_key(&empty_headers, &config));

    let mut wrong_headers = HeaderMap::new();
    wrong_headers.insert("x-api-key", "wrong_key".parse().unwrap());
    assert!(!has_valid_master_api_key(&wrong_headers, &config));

    let mut unrelated_headers = HeaderMap::new();
    unrelated_headers.insert("x-not-api-key", "master_key".parse().unwrap());
    assert!(!has_valid_master_api_key(&unrelated_headers, &config));
}

#[test]
fn is_production_uses_environment_flag() {
    assert!(is_production(&test_config("production")));
    assert!(!is_production(&test_config("test")));
    assert!(!is_production(&test_config("staging")));
}

#[test]
fn health_ok_model_contains_expected_values() {
    let health = Health::ok();

    assert_eq!(health.status, "ok");
    assert!(!health.version.is_empty());
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
}

use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    key: &'static str,
    original_value: Option<String>,
}

impl EnvGuard {
    fn new(key: &'static str, new_value: Option<&str>) -> Self {
        let original_value = std::env::var(key).ok();
        match new_value {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
        Self {
            key,
            original_value,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(ref value) = self.original_value {
            unsafe { std::env::set_var(self.key, value) };
        } else {
            unsafe { std::env::remove_var(self.key) };
        }
    }
}

#[test]
fn app_config_from_env_fails_when_master_api_key_missing() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard = EnvGuard::new("MASTER_API_KEY", None);

    let result = AppConfig::from_env();
    assert!(result.is_err());
}
