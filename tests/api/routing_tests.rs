use axum::http::StatusCode;

use crate::common::{create_test_app, get, send_json};

#[tokio::test]
async fn unknown_route_uses_structured_not_found_response() {
    // Router fallback should provide stable JSON format for unknown paths.
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/definitely-not-a-real-route")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(
        body["message"],
        "No route found for '/definitely-not-a-real-route'"
    );
    assert_eq!(body.as_object().unwrap().len(), 2);
}
