use std::env;

/// Helper: Fetches an env var, applies a default, and parses to the required type.
fn env_or<T: std::str::FromStr>(key: &str, default: &str) -> T {
    env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse::<T>()
        .unwrap_or_else(|_| panic!("{} must be a valid {}", key, std::any::type_name::<T>()))
}

/// Helper: Fetches an optional env var and trims it, returning None if empty.
fn env_opt(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub allowed_origins: Option<String>,
    pub download_dir: String,
    pub ytdlp_path: String,
    pub ytdlp_cookies_file: Option<String>,
    pub ytdlp_extractor_args: Option<String>,
    pub ytdlp_pot_provider_url: Option<String>,
    pub max_concurrent_downloads: usize,
    pub captcha_secret_key: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            name: env_or("APP_NAME", "nadzu-backend"),
            env: env_or("APP_ENV", "production"),
            host: env_or("APP_HOST", "127.0.0.1"),
            port: env_or("APP_PORT", "8080"),
            allowed_origins: env_opt("ALLOWED_ORIGINS"),
            download_dir: env_or("DOWNLOAD_DIR", "downloads"),
            ytdlp_path: env_or("YTDLP_PATH", "yt-dlp"),
            ytdlp_cookies_file: env_opt("YTDLP_COOKIES_FILE"),
            ytdlp_extractor_args: env_opt("YTDLP_EXTRACTOR_ARGS"),
            ytdlp_pot_provider_url: env_opt("YTDLP_POT_PROVIDER_URL"),
            max_concurrent_downloads: env_or("MAX_CONCURRENT_DOWNLOADS", "3"),
            captcha_secret_key: env_opt("CAPTCHA_SECRET_KEY"),
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
