use axum::http::StatusCode;

use crate::common::{EXPECTED_ROOT_MESSAGE, create_test_app, get, send_json};

#[tokio::test]
async fn root_endpoint_returns_alive_message() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], EXPECTED_ROOT_MESSAGE);
}
