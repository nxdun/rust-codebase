use axum::http::StatusCode;

use crate::common::{
    HEADER_API_KEY, TEST_MASTER_API_KEY, create_test_app, get_with_headers, send_json,
};

#[tokio::test]
async fn list_jobs_requires_api_key_when_missing() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get_with_headers("/api/v1/ytdlp/jobs", &[])).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error_code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn list_jobs_requires_api_key_when_invalid() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        get_with_headers(
            "/api/v1/ytdlp/jobs",
            &[(HEADER_API_KEY, "not_the_master_key")],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error_code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn list_jobs_returns_array_with_valid_api_key() {
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
    assert!(body["jobs"].is_array());
    assert_eq!(body["jobs"].as_array().unwrap().len(), 0);
}
