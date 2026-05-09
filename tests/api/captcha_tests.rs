use axum::http::StatusCode;

use crate::common::{
    HEADER_API_KEY, SAMPLE_YTDLP_URL, TEST_MASTER_API_KEY, create_test_app, post_json_with_headers,
    send_json, ytdlp_enqueue_request,
};

#[tokio::test]
async fn valid_api_key_bypasses_captcha_and_secret_requirement() {
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            &ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(HEADER_API_KEY, TEST_MASTER_API_KEY)],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], "accepted");
}

#[tokio::test]
async fn invalid_api_key_does_not_bypass_captcha() {
    let app = create_test_app(Some("secret"));

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            &ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(HEADER_API_KEY, "bad_key")],
        ),
    )
    .await;

    // It should fail with Validation (422) because the API key is invalid, so it tries captcha, but token is missing.
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error_code"], "VALIDATION_ERROR");
    assert_eq!(body["message"], "x-captcha-token header is required");
}
