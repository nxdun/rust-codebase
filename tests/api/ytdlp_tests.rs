use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use nadzu::routes::create_router;
use tower::ServiceExt;

use crate::common::{
    build_post_json, create_test_app, create_test_state, get_json_body, seed_ytdlp_job,
    ytdlp_enqueue_request,
};

#[tokio::test]
async fn ytdlp_enqueue_requires_captcha_header() {
    let app = create_test_app(Some("secret"));

    let response = app
        .oneshot(build_post_json(
            "/api/v1/ytdlp",
            ytdlp_enqueue_request("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = get_json_body(response).await;
    assert_eq!(body["message"], "x-captcha-token header is required");
}

#[tokio::test]
async fn ytdlp_enqueue_fails_when_secret_key_missing() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ytdlp")
                .header("content-type", "application/json")
                .header("x-captcha-token", "token")
                .body(Body::from(
                    ytdlp_enqueue_request("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = get_json_body(response).await;
    assert_eq!(body["message"], "CAPTCHA_SECRET_KEY is not configured");
}

#[tokio::test]
async fn ytdlp_enqueue_fails_when_secret_key_empty() {
    let app = create_test_app(Some("   "));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ytdlp")
                .header("content-type", "application/json")
                .header("x-captcha-token", "token")
                .body(Body::from(
                    ytdlp_enqueue_request("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = get_json_body(response).await;
    assert_eq!(body["message"], "CAPTCHA_SECRET_KEY is not configured");
}

#[tokio::test]
async fn ytdlp_list_jobs_returns_array() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ytdlp/jobs")
                .header("x-api-key", "test_master_key")
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
async fn ytdlp_get_job_returns_seeded_job() {
    let state = create_test_state(None);
    let app = create_router(state.clone()).with_state(state.clone());
    let job_id = seed_ytdlp_job(&state, "https://www.youtube.com/watch?v=dQw4w9WgXcQ").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/ytdlp/jobs/{job_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_json_body(response).await;
    assert_eq!(body["job"]["id"], job_id);
    assert_eq!(
        body["job"]["url"],
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );
}

#[tokio::test]
async fn ytdlp_get_job_not_found_returns_404() {
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
async fn ytdlp_download_file_not_found_returns_404() {
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

// Unit tests moved from mod.rs

#[test]
fn resolve_mp4_best_selector() {
    let (format_flag, sort_flag) = nadzu::services::ytdlp::resolve_format_selector("mp4", "best");
    assert_eq!(format_flag, "bv*+ba/b");
    assert_eq!(
        sort_flag,
        Some("res,vcodec:h264,acodec:aac,ext:mp4:m4a".to_string())
    );
}

#[test]
fn resolve_audio_only_selector() {
    let (format_flag, sort_flag) = nadzu::services::ytdlp::resolve_format_selector("mp4", "audio");
    assert_eq!(format_flag, "ba/b");
    assert_eq!(sort_flag, None);
}

#[test]
fn resolve_custom_selector() {
    let (format_flag, sort_flag) =
        nadzu::services::ytdlp::resolve_format_selector("custom:bestvideo+bestaudio/best", "best");
    assert_eq!(format_flag, "bestvideo+bestaudio/best");
    assert_eq!(sort_flag, None);
}
