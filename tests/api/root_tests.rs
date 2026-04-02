use axum::http::StatusCode;

use crate::common::{create_test_app, get, send};

#[tokio::test]
async fn root_endpoint_redirects_to_docs() {
    let app = create_test_app(None);

    let response = send(&app, get("/")).await;

    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "https://nadzu.me"
    );
}
