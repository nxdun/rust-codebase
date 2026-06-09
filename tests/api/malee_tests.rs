use crate::common::{
    HEADER_API_KEY, TEST_MASTER_API_KEY, create_test_app, post_json, post_json_with_headers,
    send_json,
};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn malee_chat_requires_api_key() {
    let app = create_test_app(None);
    let req = post_json("/api/v1/malee/chat", &json!({"message": "hello"}));

    let (status, _) = send_json(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn malee_chat_validates_input_length() {
    let app = create_test_app(None);
    let long_message = "a".repeat(2001);
    let req = post_json_with_headers(
        "/api/v1/malee/chat",
        &json!({"message": long_message}),
        &[(HEADER_API_KEY, TEST_MASTER_API_KEY)],
    );

    let (status, body) = send_json(&app, req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error_code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn malee_session_management() {
    let app = create_test_app(None);

    // 1. Get non-existent session
    let (status, _) = send_json(
        &app,
        crate::common::get_with_headers(
            "/api/v1/malee/session/00000000-0000-0000-0000-000000000000",
            &[(HEADER_API_KEY, TEST_MASTER_API_KEY)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 2. Track non-existent order
    let req = post_json_with_headers(
        "/api/v1/malee/track",
        &json!({"order_id": "ORD-MISSING"}),
        &[(HEADER_API_KEY, TEST_MASTER_API_KEY)],
    );
    let (status, _) = send_json(&app, req).await;
    // Connector will fail because it's pointed to a fake URL in tests
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}
