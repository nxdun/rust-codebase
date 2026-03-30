use axum::http::StatusCode;
use http_body_util::BodyExt;
use serde_json::Value;

use crate::common::{
    API_KEY_HEADER, EXPECTED_ROOT_MESSAGE, TEST_MASTER_API_KEY, create_test_app_with_rate_limit,
    get, get_with_headers, send, send_json, send_text,
};

#[tokio::test]
async fn normal_tier_hits_limit_without_api_key() {
    let app = create_test_app_with_rate_limit(None);
    let mut throttle_body: Option<Value> = None;

    for _ in 0..40 {
        let response = send(&app, get("/")).await;

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            let body = response.into_body().collect().await.unwrap().to_bytes();
            throttle_body = Some(serde_json::from_slice(&body).unwrap());
            break;
        }

        assert_eq!(response.status(), StatusCode::OK);
    }

    let throttle_body = throttle_body.expect("expected normal tier to hit limit");
    assert_eq!(throttle_body["message"], "Rate limit exceeded");
    assert_eq!(throttle_body["tier"], "normal");
}

#[tokio::test]
async fn enhanced_tier_allows_higher_burst_with_valid_api_key() {
    let app = create_test_app_with_rate_limit(None);

    for _ in 0..40 {
        let (status, body) = send_text(
            &app,
            get_with_headers("/", &[(API_KEY_HEADER, TEST_MASTER_API_KEY)]),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, EXPECTED_ROOT_MESSAGE);
    }
}

#[tokio::test]
async fn health_endpoint_is_not_rate_limited() {
    let app = create_test_app_with_rate_limit(None);

    let mut validated_payload_contract = false;

    for _ in 0..80 {
        let (status, body) = send_json(&app, get("/health")).await;
        assert_eq!(status, StatusCode::OK);

        if !validated_payload_contract {
            assert_eq!(body["status"], "ok");
            assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
            validated_payload_contract = true;
        }
    }

    assert!(validated_payload_contract);
}
