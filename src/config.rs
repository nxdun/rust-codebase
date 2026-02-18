use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub allowed_origins: Option<String>,
}

// Configuration struct for the application
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
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
