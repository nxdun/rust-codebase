use crate::config::AppConfig;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use tracing::info;

pub type RedisPool = Pool<RedisConnectionManager>;

pub async fn init_redis() -> RedisPool {
    let cfg = AppConfig::from_env().expect("Failed to load config");
    info!("Connecting to redis...");

    let manager = RedisConnectionManager::new(cfg.redis_url)
        .expect("Failed to create Redis connection manager");
        
    let pool = Pool::builder()
        .build(manager)
        .await
        .expect("Failed to create Redis pool");

    info!("Redis connection established successfully");
    pool
}