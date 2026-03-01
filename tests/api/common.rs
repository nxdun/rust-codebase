use axum::{
    body::Body,
    http::{Request, Response},
};
use http_body_util::BodyExt;
use nadzu::{
    config::AppConfig,
    models::ytdlp_model::YtdlpDownloadRequest,
    routes::create_router,
    services::ytdlp::YtdlpManager,
    state::AppState,
};
use serde_json::{Value, json};
use std::sync::Arc;

//test Configuration
pub fn create_test_state(secret_key: Option<&str>) -> AppState {
    let config = Arc::new(AppConfig {
        name: "test".into(),
        env: "test".into(),
        host: "127.0.0.1".into(),
        port: 8080,
        allowed_origins: None,
        download_dir: "downloads".into(),
        ytdlp_path: "yt-dlp".into(),
        ytdlp_cookies_file: None,
        ytdlp_extractor_args: None,
        ytdlp_pot_provider_url: None,
        max_concurrent_downloads: 3,
        captcha_secret_key: secret_key.map(str::to_string),
    });

    let ytdlp_manager = Arc::new(YtdlpManager::new(config.clone()));
    let http_client = reqwest::Client::new();

    AppState {
        config,
        ytdlp_manager,
        http_client,
    }
}

pub fn create_test_app(secret_key: Option<&str>) -> axum::Router {
    let state = create_test_state(secret_key);
    create_router(state.clone()).with_state(state)
}

pub async fn get_json_body(response: Response<Body>) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

pub fn ytdlp_enqueue_request(url: &str) -> Value {
    json!({
        "url": url,
        "quality": "best",
        "format": "mp4"
    })
}

pub async fn seed_ytdlp_job(state: &AppState, url: &str) -> String {
    let job = state
        .ytdlp_manager
        .enqueue_download(YtdlpDownloadRequest {
            url: url.to_string(),
            quality: Some("best".into()),
            format: Some("mp4".into()),
            folder: None,
        })
        .await;

    job.id
}

pub fn build_post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}
