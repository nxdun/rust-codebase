use axum::http::StatusCode;

use crate::common::{
    API_KEY_HEADER, SAMPLE_YTDLP_URL, TEST_MASTER_API_KEY, create_test_app, post_json_with_headers,
    send_json, ytdlp_enqueue_request,
};

#[tokio::test]
async fn valid_api_key_bypasses_captcha_and_secret_requirement() {
    // A trusted API key should bypass CAPTCHA checks, even when CAPTCHA secret is unset.
    let app = create_test_app(None);

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(API_KEY_HEADER, TEST_MASTER_API_KEY)],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], "accepted");
    assert_eq!(body["message"], "Download enqueued");
    assert_eq!(body["job"]["url"], SAMPLE_YTDLP_URL);
    assert_eq!(body["job"]["status"], "queued");
    assert!(body["job"]["id"].as_str().unwrap().starts_with("ytdlp-"));
}

#[tokio::test]
async fn invalid_api_key_does_not_bypass_captcha() {
    // A wrong API key should not bypass CAPTCHA protection and should still require token header.
    let app = create_test_app(Some("secret"));

    let (status, body) = send_json(
        &app,
        post_json_with_headers(
            "/api/v1/ytdlp",
            ytdlp_enqueue_request(SAMPLE_YTDLP_URL),
            &[(API_KEY_HEADER, "bad_key")],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "x-captcha-token header is required");
    assert!(body.get("status").is_none());
}
