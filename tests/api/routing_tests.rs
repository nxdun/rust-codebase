use axum::http::StatusCode;

use crate::common::{create_test_app, get, send_json};

#[tokio::test]
async fn unknown_route_uses_structured_not_found_response() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/definitely-not-a-real-route")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(body["error_code"], "NOT_FOUND");
    assert!(body["message"].as_str().unwrap().contains("No route found"));
}
