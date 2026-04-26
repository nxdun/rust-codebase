use axum::http::StatusCode;
use serde_json::json;

use crate::common::{create_test_app, post_json, post_raw_json_with_headers, send_json};

#[tokio::test]
async fn validate_user_accepts_valid_payload() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json(
            "/validate-user",
            &json!({
                "name": "nadun",
                "email": "nadun@example.com",
                "age": 25
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn validate_user_rejects_malformed_json() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_raw_json_with_headers("/validate-user", "{\"name\":\"abc\",\"email\":", &[]),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["status"], 422);
    assert_eq!(body["error_code"], "VALIDATION_ERROR");
    assert_eq!(body["message"], "Invalid JSON format");
}

#[tokio::test]
async fn validate_user_rejects_semantically_invalid_payload() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json(
            "/validate-user",
            &json!({
                "name": "ab",
                "email": "not-email",
                "age": 15
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["status"], 422);
    assert_eq!(body["error_code"], "VALIDATION_ERROR");
    assert!(body["message"].as_str().unwrap().contains("messages"));
}
