use axum::http::StatusCode;
use nadzu::routes::create_router;
use serde_json::json;

use crate::common::{
    API_KEY_HEADER, CAPTCHA_TOKEN_HEADER, SAMPLE_YTDLP_URL, TEST_MASTER_API_KEY, create_test_app,
    create_test_state, get, get_with_headers, post_json, post_json_with_headers, seed_ytdlp_job,
    send_json, ytdlp_enqueue_request,
};

fn validation_error_for_field<'a>(errors: &'a [serde_json::Value], field: &str) -> &'a serde_json::Value {
    errors
        .iter()
        .find(|entry| entry["field"] == field)
        .expect("expected validation error for field")
}

#[tokio::test]
async fn ytdlp_enqueue_requires_captcha_header() {
    let app = create_test_app(Some("secret"));

    let (status, body) = send_json(
        &app,
        post_json("/api/v1/ytdlp", ytdlp_enqueue_request(SAMPLE_YTDLP_URL)),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "x-captcha-token header is required");
    assert_eq!(body.as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn ytdlp_enqueue_fails_when_secret_key_missing() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(CAPTCHA_TOKEN_HEADER, "token")],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["message"], "CAPTCHA_SECRET_KEY is not configured");
    assert_eq!(body.as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn ytdlp_enqueue_fails_when_secret_key_empty() {
    let app = create_test_app(Some("   "));

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(CAPTCHA_TOKEN_HEADER, "token")],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["message"], "CAPTCHA_SECRET_KEY is not configured");
    assert_eq!(body.as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn ytdlp_list_jobs_returns_array() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        get_with_headers("/api/v1/ytdlp/jobs", &[(API_KEY_HEADER, TEST_MASTER_API_KEY)]),
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
    assert_eq!(body["job"]["id"], job_id);
    assert_eq!(body["job"]["url"], SAMPLE_YTDLP_URL);
    assert_eq!(body["job"]["status"], "queued");
    assert!(!body["job"]["output_dir"].as_str().unwrap().is_empty());
    assert!(!body["job"]["format_flag"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn ytdlp_get_job_not_found_returns_404() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/jobs/nonexistent_id")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(body["message"], "job not found");
}

#[tokio::test]
async fn ytdlp_download_file_not_found_returns_404() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/download/nonexistent_id")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "Job not found");
}

#[tokio::test]
async fn ytdlp_stream_progress_not_found_returns_404() {
    // Streaming endpoint should fail fast when the job id is unknown.
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/jobs/nonexistent_id/stream")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(body["message"], "job not found");
}

#[tokio::test]
async fn ytdlp_supported_sites_returns_service_unavailable_without_generated_file() {
    // On local/dev test environments, sites.txt is absent and endpoint should return 503.
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/api/v1/ytdlp/sites")).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body["error"].is_string());
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("missing")
            || body["error"].as_str().unwrap().contains("not generated")
    );
}

#[tokio::test]
async fn ytdlp_enqueue_rejects_invalid_url_payload() {
    // Extractor validation should reject malformed URL payloads before enqueueing jobs.
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            json!({
                "url": "not-a-url",
                "quality": "best",
                "format": "mp4"
            }),
            &[(API_KEY_HEADER, TEST_MASTER_API_KEY)],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["status"], 422);
    assert_eq!(body["message"], "Validation failed");

    let errors = body["errors"].as_array().unwrap();
    let url_error = validation_error_for_field(errors, "url");
    assert!(!url_error["messages"].as_array().unwrap().is_empty());
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
