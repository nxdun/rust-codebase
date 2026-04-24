#![allow(unsafe_code)]
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
        github_pat: None,
        github_username: None,
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn has_valid_master_api_key_returns_true_for_matching_header() {
    // Utility layer contract: header parser and key matcher should accept valid key.
    let config = test_config("test");
    let mut headers = HeaderMap::new();
    headers.insert(API_KEY_HEADER, "master_key".parse().unwrap());

    assert!(has_valid_master_api_key(&headers, &config));
}

#[test]
#[allow(clippy::unwrap_used)]
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
fn app_config_from_env_exits_when_master_api_key_missing() -> std::io::Result<()> {
    let helper_binary = env!("CARGO_BIN_EXE_config_exit");

    let output = std::process::Command::new(helper_binary)
        .env_remove("MASTER_API_KEY")
        .output()?;

    assert!(
        !output.status.success(),
        "expected exit status to be non-zero"
    );
    assert_eq!(output.status.code(), Some(1));

    Ok(())
}
