use axum::http::{Method, StatusCode};

use crate::common::{create_test_app_with_full_layers, empty_request, send};

#[tokio::test]
async fn cors_preflight_allows_required_custom_headers() {
    // CORS preflight must explicitly allow custom security headers used by middleware.
    let app = create_test_app_with_full_layers(None, Some("http://localhost:5173"));

    let response = send(
        &app,
        empty_request(
            Method::OPTIONS,
            "/health",
            &[
                ("origin", "http://localhost:5173"),
                ("access-control-request-method", "GET"),
                ("access-control-request-headers", "x-api-key,x-captcha-token"),
            ],
        ),
    )
    .await;

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NO_CONTENT,
        "unexpected preflight status: {}",
        response.status()
    );

    let origin = response
        .headers()
        .get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert_eq!(origin, "http://localhost:5173");

    let allow_headers = response
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_ascii_lowercase();
    assert!(allow_headers.contains("x-api-key"));
    assert!(allow_headers.contains("x-captcha-token"));

    let allow_methods = response
        .headers()
        .get("access-control-allow-methods")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_ascii_uppercase();
    assert!(allow_methods.contains("GET"));
    assert!(allow_methods.contains("POST"));

    let allow_credentials = response
        .headers()
        .get("access-control-allow-credentials")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert_eq!(allow_credentials, "true");
}
