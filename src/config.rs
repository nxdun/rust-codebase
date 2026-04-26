use std::env;
use thiserror::Error;

use crate::middleware::constant_time_eq;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingVar(String),
    #[error("Invalid value for {key}: {details}")]
    InvalidValue { key: String, details: String },
}

/// Helper: Fetches an env var, applies a default, and parses to the required type.
fn env_or<T: std::str::FromStr>(key: &str, default: &str) -> T {
    env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse::<T>()
        .unwrap_or_else(|_| {
            tracing::error!("{} must be a valid {}", key, std::any::type_name::<T>());
            #[allow(clippy::expect_used)]
            default
                .to_string()
                .parse::<T>()
                .ok()
                .expect("default must be valid")
        })
}

/// Helper: Fetches an optional env var and trims it, returning None if empty.
fn env_opt(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub allowed_origins: Option<String>,
    pub download_dir: String,
    pub ytdlp_path: String,
    pub ytdlp_external_downloader: Option<String>,
    pub ytdlp_external_downloader_args: Option<String>,
    pub max_concurrent_downloads: usize,
    pub captcha_secret_key: Option<String>,
    master_api_key: String, // Private: use check_api_key
    pub github_pat: Option<String>,
    pub github_username: Option<String>,
    pub github_graphql_url: String,
}

impl AppConfig {
    /// Internal constructor for creating config instances.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        name: String,
        env: String,
        host: String,
        port: u16,
        allowed_origins: Option<String>,
        download_dir: String,
        ytdlp_path: String,
        ytdlp_external_downloader: Option<String>,
        ytdlp_external_downloader_args: Option<String>,
        max_concurrent_downloads: usize,
        captcha_secret_key: Option<String>,
        master_api_key: String,
        github_pat: Option<String>,
        github_username: Option<String>,
        github_graphql_url: String,
    ) -> Self {
        Self {
            name,
            env,
            host,
            port,
            allowed_origins,
            download_dir,
            ytdlp_path,
            ytdlp_external_downloader,
            ytdlp_external_downloader_args,
            max_concurrent_downloads,
            captcha_secret_key,
            master_api_key,
            github_pat,
            github_username,
            github_graphql_url,
        }
    }

    /// Loads the application configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let master_api_key = env_opt("MASTER_API_KEY")
            .ok_or_else(|| ConfigError::MissingVar("MASTER_API_KEY".to_string()))?;

        Ok(Self::new(
            env_or("APP_NAME", "nadzu-backend"),
            env_or("APP_ENV", "production"),
            env_or("APP_HOST", "127.0.0.1"),
            env_or("APP_PORT", "8080"),
            env_opt("ALLOWED_ORIGINS"),
            env_or("DOWNLOAD_DIR", "downloads"),
            env_or("YTDLP_PATH", "yt-dlp"),
            env_opt("YTDLP_EXTERNAL_DOWNLOADER"),
            env_opt("YTDLP_EXTERNAL_DOWNLOADER_ARGS"),
            env_or("MAX_CONCURRENT_DOWNLOADS", "3"),
            env_opt("CAPTCHA_SECRET_KEY"),
            master_api_key,
            env_opt("GITHUB_PAT"),
            env_opt("GITHUB_USERNAME"),
            env_or("GITHUB_GRAPHQL_URL", "https://api.github.com/graphql"),
        ))
    }

    /// Securely checks if the provided key matches the master API key using constant-time comparison.
    #[must_use]
    pub fn check_api_key(&self, provided_key: &str) -> bool {
        constant_time_eq(provided_key, &self.master_api_key)
    }

    /// Returns the full address string for the server.
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Helper for testing to inject a master API key.
    #[cfg(test)]
    #[must_use]
    pub fn with_master_key(mut self, key: &str) -> Self {
        self.master_api_key = key.to_string();
        self
    }
}
