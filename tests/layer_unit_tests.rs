use axum::http::HeaderMap;
use nadzu::{
    config::AppConfig,
    middleware::{
        api_key::{API_KEY_HEADER, has_valid_master_api_key},
        rate_limit::is_production,
    },
    models::health_model::Health,
};

fn test_config(env: &str) -> AppConfig {
    AppConfig {
        name: "nadzu-test".to_string(),
        env: env.to_string(),
        host: "127.0.0.1".to_string(),
        port: 8080,
        allowed_origins: None,
        download_dir: "downloads".to_string(),
        ytdlp_path: "yt-dlp".to_string(),
        ytdlp_external_downloader: None,
        ytdlp_external_downloader_args: None,
        max_concurrent_downloads: 3,
        captcha_secret_key: None,
        master_api_key: "master_key".to_string(),
    }
}

#[test]
fn has_valid_master_api_key_returns_true_for_matching_header() {
    // Utility layer contract: header parser and key matcher should accept valid key.
    let config = test_config("test");
    let mut headers = HeaderMap::new();
    headers.insert(API_KEY_HEADER, "master_key".parse().unwrap());

    assert!(has_valid_master_api_key(&headers, &config));
}

#[test]
fn has_valid_master_api_key_returns_false_for_missing_or_wrong_header() {
    // Utility layer contract: missing/wrong keys must be rejected consistently.
    let config = test_config("test");

    let empty_headers = HeaderMap::new();
    assert!(!has_valid_master_api_key(&empty_headers, &config));

    let mut wrong_headers = HeaderMap::new();
    wrong_headers.insert(API_KEY_HEADER, "wrong_key".parse().unwrap());
    assert!(!has_valid_master_api_key(&wrong_headers, &config));

    let mut unrelated_headers = HeaderMap::new();
    unrelated_headers.insert("x-not-api-key", "master_key".parse().unwrap());
    assert!(!has_valid_master_api_key(&unrelated_headers, &config));
}

#[test]
fn is_production_uses_environment_flag() {
    // Rate-limit policy helper should only treat literal production env as production.
    assert!(is_production(&test_config("production")));
    assert!(!is_production(&test_config("test")));
    assert!(!is_production(&test_config("staging")));
}

#[test]
fn health_ok_model_contains_expected_values() {
    // Model layer contract for health payload should remain stable.
    let health = Health::ok();

    assert_eq!(health.status, "ok");
    assert!(!health.version.is_empty());
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
}

#[test]
#[should_panic(expected = "MASTER_API_KEY must be set")]
fn app_config_from_env_panics_when_master_api_key_missing() {
    let original_key = std::env::var("MASTER_API_KEY").ok();
    unsafe { std::env::remove_var("MASTER_API_KEY") };

    let _config = AppConfig::from_env();

    if let Some(key) = original_key {
        unsafe { std::env::set_var("MASTER_API_KEY", key) };
    }
}
