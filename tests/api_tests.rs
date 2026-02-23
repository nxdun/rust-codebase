use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use nadzu::{
    config::AppConfig, routes::create_router, services::ytdlp::YtdlpManager, state::AppState,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

fn create_test_state(secret_key: Option<String>) -> AppState {
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
        captcha_secret_key: secret_key,
    });
    let ytdlp_manager = Arc::new(YtdlpManager::new(config.clone()));
    let http_client = reqwest::Client::new();
    AppState {
        config,
        ytdlp_manager,
        http_client,
    }
}

fn create_test_app(secret_key: Option<String>) -> axum::Router {
    let state = create_test_state(secret_key);
    create_router().with_state(state)
}

async fn get_json_body(response: axum::http::Response<Body>) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn test_root_endpoint() {
    let app = create_test_app(None);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, "test - alive and listening");
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_json_body(response).await;
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_verify_captcha_empty_token() {
    let app = create_test_app(Some("secret".into()));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/captcha/verify")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "captcha": "   " }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = get_json_body(response).await;
    assert_eq!(body["message"], "CapToken is required");
}

#[tokio::test]
async fn test_verify_captcha_missing_token() {
    let app = create_test_app(Some("secret".into()));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/captcha/verify")
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = get_json_body(response).await;
    assert_eq!(body["message"], "CapToken is required");
}

#[tokio::test]
async fn test_verify_captcha_missing_secret_key() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/captcha/verify")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "captcha": "valid_token" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = get_json_body(response).await;
    assert_eq!(body["message"], "CAPTCHA_SECRET_KEY is not configured");
}

#[tokio::test]
async fn test_verify_captcha_empty_secret_key() {
    let app = create_test_app(Some("   ".into()));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/captcha/verify")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "captcha": "valid_token" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = get_json_body(response).await;
    assert_eq!(body["message"], "CAPTCHA_SECRET_KEY is not configured");
}

#[tokio::test]
async fn test_ytdlp_enqueue_watch_url() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ytdlp")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                        "quality": "best",
                        "format": "mp4"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = get_json_body(response).await;
    assert!(body.get("job").is_some());
    assert!(body["job"].get("id").is_some());
    assert_eq!(
        body["job"]["url"],
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );
}

#[tokio::test]
async fn test_ytdlp_enqueue_shorts_url() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ytdlp")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "url": "https://youtube.com/shorts/5g4pLlLH6P4?si=jaO5XHPymDBSc5uL",
                        "quality": "best",
                        "format": "mp4"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = get_json_body(response).await;
    assert!(body.get("job").is_some());
    assert!(body["job"].get("id").is_some());
    assert!(
        body["job"]["url"]
            .as_str()
            .unwrap()
            .contains("https://www.youtube.com/watch?v=")
    );
}

#[tokio::test]
async fn test_ytdlp_list_jobs() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ytdlp/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_json_body(response).await;
    assert!(body.get("jobs").is_some());
    assert!(body["jobs"].is_array());
}

#[tokio::test]
async fn test_ytdlp_get_job_success() {
    let app = create_test_app(None);

    // 1. Enqueue a job
    let enqueue_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ytdlp")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                        "quality": "best",
                        "format": "mp4"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(enqueue_response.status(), StatusCode::ACCEPTED);
    let enqueue_body = get_json_body(enqueue_response).await;
    let job_id = enqueue_body["job"]["id"].as_str().unwrap().to_string();

    // 2. Get the job
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/ytdlp/jobs/{}", job_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = get_json_body(get_response).await;
    assert_eq!(get_body["job"]["id"], job_id);
}

#[tokio::test]
async fn test_ytdlp_get_job_not_found() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ytdlp/jobs/nonexistent_id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_ytdlp_download_file_not_found() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ytdlp/download/nonexistent_id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
