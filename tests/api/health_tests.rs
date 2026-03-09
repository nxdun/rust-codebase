use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use nadzu::routes::create_router;
use std::fs;
use tower::ServiceExt;

use crate::common::{create_test_app, create_test_state_with, get_json_body};

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = create_test_app(None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_json_body(response).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["cookies"], false);
}

#[tokio::test]
async fn health_endpoint_reports_cookies_true_when_file_exists() {
    let tmp_path = std::env::temp_dir().join("nadzu-health-cookies-test.txt");
    fs::write(&tmp_path, "# cookies").unwrap();

    let state = create_test_state_with(None, Some(tmp_path.to_string_lossy().as_ref()));
    let app = create_router(state.clone()).with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_json_body(response).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["cookies"], true);

    let _ = fs::remove_file(tmp_path);
}
