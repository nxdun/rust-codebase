use apalis::prelude::*;
use shared_core::db::postgres::{init_db, DbPool};
use shared_core::models::ytdlp::DownloadJob;
use shared_core::config::AppConfig;
use tokio::time::sleep;
use std::time::Duration;
use tracing::info;

// Use apalis::Error if available, otherwise fallback
type ApalisError = BoxDynError;

async fn handle_download(job: DownloadJob, db: Data<DbPool>) -> Result<(), ApalisError> {
    info!("Processing job: {} for URL: {}", job.id, job.request.url);
    
    // 1. Await a tokio::time::sleep of 2 seconds (mocking cold start).
    sleep(Duration::from_secs(2)).await;

    // 2. Update the job status in PostgreSQL to 'Processing'.
    let pool = &*db;
    sqlx::query("UPDATE jobs SET status = $1 WHERE id = $2")
        .bind("Processing")
        .bind(&job.id)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {e}"))?;

    // 3. Use tokio::process::Command to execute echo {job.url}.
    let output = tokio::process::Command::new("echo")
        .arg(&job.request.url)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Process error: {e}"))?;
    
    info!("Mock Execution Output: {}", String::from_utf8_lossy(&output.stdout).trim());

    // 4. Update the job status in PostgreSQL to 'Completed' with a mock S3 URL.
    sqlx::query("UPDATE jobs SET status = $1, error = $2 WHERE id = $3")
        .bind("Completed")
        .bind("https://mock-s3-bucket.s3.amazonaws.com/video.mp4")
        .bind(&job.id)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {e}"))?;

    info!("Job finished successfully: {}", job.id);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Backmatter Worker starting...");

    // Load config
    let cfg = AppConfig::from_env().expect("Failed to load configuration");
    
    // 1. Connect to PostgreSQL
    let db_pool = init_db().await;
    
    // 2. Connect to Redis for apalis
    let conn = apalis_redis::connect(cfg.redis_url).await.expect("Failed to connect to Redis for apalis");
    let storage = apalis_redis::RedisStorage::new(conn);

    info!("Worker connected to Redis and PostgreSQL. Polling for jobs...");

    // 3. Set up an apalis::Monitor that continuously polls the Redis queue
    Monitor::new()
        .register(
            WorkerBuilder::new("ytdlp-worker")
                .data(db_pool)
                .backend(storage)
                .build_fn(handle_download)
        )
        .run()
        .await?;

    Ok(())
}
