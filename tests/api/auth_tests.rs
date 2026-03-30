use axum::http::StatusCode;

use crate::common::{
    API_KEY_HEADER, TEST_MASTER_API_KEY, create_test_app, get, get_with_headers, send_text,
    send_json,
};

#[tokio::test]
async fn list_jobs_requires_api_key_when_missing() {
    // Requests without x-api-key must be rejected by auth middleware.
    let app = create_test_app(None);

    let (status, body) = send_text(&app, get("/api/v1/ytdlp/jobs")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        body.is_empty() || body.to_ascii_lowercase().contains("unauthorized"),
        "unexpected unauthorized body: {body}"
    );
}

#[tokio::test]
async fn list_jobs_requires_api_key_when_invalid() {
    // Invalid credentials should not pass the protected listing endpoint.
    let app = create_test_app(None);

    let (status, body) = send_text(
        &app,
        get_with_headers("/api/v1/ytdlp/jobs", &[(API_KEY_HEADER, "not_the_master_key")]),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        body.is_empty() || body.to_ascii_lowercase().contains("unauthorized"),
        "unexpected unauthorized body: {body}"
    );
}

#[tokio::test]
async fn list_jobs_returns_array_with_valid_api_key() {
    // Valid API key should unlock the endpoint and return the expected payload shape.
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        get_with_headers("/api/v1/ytdlp/jobs", &[(API_KEY_HEADER, TEST_MASTER_API_KEY)]),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["jobs"].is_array());
    assert_eq!(body["jobs"].as_array().unwrap().len(), 0);
}
