use axum::http::StatusCode;

use crate::common::{create_test_app, get, send_json};

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = create_test_app(None);

    let (status, body) = send_json(&app, get("/health")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body.as_object().unwrap().len(), 2);
}
