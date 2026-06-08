use crate::config::AppConfig;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::{error, info};

pub type DbPool = PgPool;

pub async fn init_db() -> DbPool {
    let cfg = AppConfig::from_env().expect("Failed to load config");
    let url = cfg.database_url;

    info!("Connecting to PostgreSQL...");

    let pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&url)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to connect to PostgreSQL: {}", e);
            panic!("Database connection failed");
        });

    info!("PostgreSQL connection established successfully");

    pool
}