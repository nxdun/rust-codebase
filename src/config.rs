use std::{env, fmt};
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

/// Helper: Fetches and parses an environment variable, returning `ConfigError::InvalidValue` on parse failure.
fn env_parse<T: std::str::FromStr>(key: &str, default: &str) -> Result<T, ConfigError> {
    let val = env::var(key).unwrap_or_else(|_| default.to_string());
    val.parse::<T>().map_err(|_| ConfigError::InvalidValue {
        key: key.to_string(),
        details: format!("'{}' is not a valid {}", val, std::any::type_name::<T>()),
    })
}

/// Helper: Fetches an optional env var and trims it, returning None if empty.
fn env_opt(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Application configuration settings loaded from the environment.
#[derive(Clone)]
pub struct AppConfig {
    /// The application name.
    pub name: String,
    /// The environment (e.g., "development", "production").
    pub env: String,
    /// The host to bind to.
    pub host: String,
    /// The port to bind to.
    pub port: u16,
    /// Allowed origins for CORS.
    pub allowed_origins: Option<String>,
    /// Directory for downloaded files.
    pub download_dir: String,
    /// Path to the yt-dlp executable.
    pub ytdlp_path: String,
    /// Optional external downloader for yt-dlp.
    pub ytdlp_external_downloader: Option<String>,
    /// Optional arguments for the external downloader.
    pub ytdlp_external_downloader_args: Option<String>,
    /// Maximum concurrent yt-dlp downloads.
    pub max_concurrent_downloads: usize,
    captcha_secret_key: Option<String>,
    master_api_key: String,
    github_pat: Option<String>,
    /// GitHub username for contributions.
    pub github_username: Option<String>,
    /// GitHub GraphQL API URL.
    pub github_graphql_url: String,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("name", &self.name)
            .field("env", &self.env)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("allowed_origins", &self.allowed_origins)
            .field("download_dir", &self.download_dir)
            .field("ytdlp_path", &self.ytdlp_path)
            .field("ytdlp_external_downloader", &self.ytdlp_external_downloader)
            .field(
                "ytdlp_external_downloader_args",
                &self.ytdlp_external_downloader_args,
            )
            .field("max_concurrent_downloads", &self.max_concurrent_downloads)
            .field("captcha_secret_key", &"***")
            .field("master_api_key", &"***")
            .field("github_pat", &"***")
            .field("github_username", &self.github_username)
            .field("github_graphql_url", &self.github_graphql_url)
            .finish()
    }
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
            env_parse("APP_PORT", "8080")?,
            env_opt("ALLOWED_ORIGINS"),
            env_or("DOWNLOAD_DIR", "downloads"),
            env_or("YTDLP_PATH", "yt-dlp"),
            env_opt("YTDLP_EXTERNAL_DOWNLOADER"),
            env_opt("YTDLP_EXTERNAL_DOWNLOADER_ARGS"),
            env_parse("MAX_CONCURRENT_DOWNLOADS", "3")?,
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

    /// Returns the GitHub Personal Access Token if configured.
    pub fn github_pat(&self) -> Option<&str> {
        self.github_pat.as_deref()
    }

    /// Returns the CAPTCHA secret key if configured.
    pub fn captcha_secret_key(&self) -> Option<&str> {
        self.captcha_secret_key.as_deref()
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
