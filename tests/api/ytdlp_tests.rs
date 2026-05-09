use axum::http::StatusCode;
use nadzu::routes::create_router;
use serde_json::json;

use crate::common::{
    CAPTCHA_TOKEN_HEADER, HEADER_API_KEY, SAMPLE_YTDLP_URL, TEST_MASTER_API_KEY, create_test_app,
    create_test_state, get, get_with_headers, post_json, post_json_with_headers, seed_ytdlp_job,
    send_json, ytdlp_enqueue_request,
};

#[tokio::test]
async fn ytdlp_enqueue_requires_captcha_header() {
    let app = create_test_app(Some("secret"));

    let (status, body) = send_json(
        &app,
        post_json("/api/v1/ytdlp", &ytdlp_enqueue_request(SAMPLE_YTDLP_URL)),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["message"], "x-captcha-token header is required");
    assert_eq!(body["error_code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn ytdlp_enqueue_fails_when_secret_key_missing() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            &ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(CAPTCHA_TOKEN_HEADER, "token")],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error_code"], "SERVICE_UNAVAILABLE");
}

#[tokio::test]
async fn ytdlp_enqueue_fails_when_secret_key_empty() {
    let app = create_test_app(Some("   "));

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            &ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(CAPTCHA_TOKEN_HEADER, "token")],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error_code"], "SERVICE_UNAVAILABLE");
}

#[tokio::test]
async fn ytdlp_list_jobs_returns_array() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        get_with_headers(
            "/api/v1/ytdlp/jobs",
            &[(HEADER_API_KEY, TEST_MASTER_API_KEY)],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.get("jobs").is_some());
    assert!(body["jobs"].is_array());
    assert_eq!(body["jobs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn ytdlp_get_job_returns_seeded_job() {
    let state = create_test_state(None);
    let app = create_router(state.clone()).with_state(state.clone());
    let job_id = seed_ytdlp_job(&state, SAMPLE_YTDLP_URL).await;
    let uri = format!("/api/v1/ytdlp/jobs/{job_id}");

    let (status, body) = send_json(&app, get(&uri)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], job_id);
    assert_eq!(body["url"], SAMPLE_YTDLP_URL);
    assert_eq!(body["status"], "queued");
}

#[tokio::test]
async fn ytdlp_get_job_not_found_returns_404() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/jobs/nonexistent_id")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(body["error_code"], "NOT_FOUND");
    assert!(body["message"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn ytdlp_download_file_not_found_returns_404() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/download/nonexistent_id")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error_code"], "NOT_FOUND");
    assert!(body["message"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn ytdlp_stream_progress_not_found_returns_404() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/jobs/nonexistent_id/stream")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(body["error_code"], "NOT_FOUND");
}

#[tokio::test]
async fn ytdlp_supported_sites_returns_service_unavailable_without_generated_file() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/sites")).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error_code"], "SERVICE_UNAVAILABLE");
}

#[tokio::test]
async fn ytdlp_enqueue_rejects_invalid_url_payload() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            &json!({
                "url": "not-a-url",
                "quality": "best",
                "format": "mp4"
            }),
            &[(HEADER_API_KEY, TEST_MASTER_API_KEY)],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["status"], 422);
    assert_eq!(body["error_code"], "VALIDATION_ERROR");
}
