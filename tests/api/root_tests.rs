use axum::http::StatusCode;

use crate::common::{EXPECTED_ROOT_MESSAGE, create_test_app, get, send_text};

#[tokio::test]
async fn root_endpoint_returns_alive_message() {
    let app = create_test_app(None);

    let (status, body) = send_text(&app, get("/")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, EXPECTED_ROOT_MESSAGE);
}
