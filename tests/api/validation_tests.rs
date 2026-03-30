use axum::http::StatusCode;
use serde_json::json;

use crate::common::{create_test_app, post_json, post_raw_json_with_headers, send_json};

fn error_entry_for_field<'a>(errors: &'a [serde_json::Value], field: &str) -> &'a serde_json::Value {
    errors
        .iter()
        .find(|entry| entry["field"] == field)
        .expect("expected field error entry")
}

#[tokio::test]
async fn validate_user_accepts_valid_payload() {
    // End-to-end happy path: extractor validation + controller response mapping.
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json(
            "/validate-user",
            json!({
                "name": "nadun",
                "email": "nadun@example.com",
                "age": 25
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Validation successful");
    assert_eq!(body["data"]["name"], "nadun");
    assert_eq!(body["data"]["email"], "nadun@example.com");
    assert_eq!(body["data"]["age"], 25);
}

#[tokio::test]
async fn validate_user_rejects_malformed_json() {
    // Extractor should guard against bad JSON syntax before reaching controller.
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_raw_json_with_headers("/validate-user", "{\"name\":\"abc\",\"email\":", &[]),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400);
    assert_eq!(body["message"], "Invalid JSON format");
    assert_eq!(body.as_object().unwrap().len(), 2);
}

#[tokio::test]
async fn validate_user_rejects_semantically_invalid_payload() {
    // Extractor should return a rich 422 response with field-level details.
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json(
            "/validate-user",
            json!({
                "name": "ab",
                "email": "not-email",
                "age": 15
            }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["status"], 422);
    assert_eq!(body["message"], "Validation failed");

    let errors = body["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 3);

    let name_error = error_entry_for_field(errors, "name");
    assert!(!name_error["messages"].as_array().unwrap().is_empty());

    let email_error = error_entry_for_field(errors, "email");
    assert!(!email_error["messages"].as_array().unwrap().is_empty());

    let age_error = error_entry_for_field(errors, "age");
    assert!(!age_error["messages"].as_array().unwrap().is_empty());
}
