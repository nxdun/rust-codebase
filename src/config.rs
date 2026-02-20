use std::env;

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
    pub ytdlp_output_template: String,
    pub ytdlp_extractor_args: Option<String>,
    pub ytdlp_pot_provider_url: Option<String>,
    pub max_concurrent_downloads: usize,
}

// Configuration struct for the application.
impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            name: env::var("APP_NAME").unwrap_or_else(|_| "nadzu_api".into()),
            env: env::var("APP_ENV").unwrap_or_else(|_| "production".into()),
            host: env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: env::var("APP_PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .unwrap(),
            allowed_origins: env::var("ALLOWED_ORIGINS").ok(),
            download_dir: env::var("DOWNLOAD_DIR").unwrap_or_else(|_| "downloads".into()),
            ytdlp_path: env::var("YTDLP_PATH").unwrap_or_else(|_| "yt-dlp".into()),
            ytdlp_cookies_file: env::var("YTDLP_COOKIES_FILE").ok(),
            ytdlp_output_template: env::var("YTDLP_OUTPUT_TEMPLATE")
                .unwrap_or_else(|_| "%(title)s.%(ext)s".into()),
            ytdlp_extractor_args: env::var("YTDLP_EXTRACTOR_ARGS").ok(),
            ytdlp_pot_provider_url: env::var("YTDLP_POT_PROVIDER_URL").ok(),
            max_concurrent_downloads: env::var("MAX_CONCURRENT_DOWNLOADS")
                .unwrap_or_else(|_| "3".into())
                .parse()
                .unwrap_or(3),
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
