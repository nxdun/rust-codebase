use axum::http::StatusCode;
use serde_json::Value;

use crate::common::{
    API_KEY_HEADER, EXPECTED_ROOT_MESSAGE, TEST_MASTER_API_KEY, create_test_app_with_rate_limit,
    get_with_headers, send_json,
};

#[tokio::test]
async fn normal_tier_hits_limit_without_api_key() {
    let app = create_test_app_with_rate_limit(None);
    let mut throttle_body: Option<Value> = None;

    for _ in 0..40 {
        let (status, body) = send_json(&app, get_with_headers("/", &[])).await;

        if status == StatusCode::FORBIDDEN {
            throttle_body = Some(body);
            break;
        }

        assert_eq!(status, StatusCode::OK);
    }

    let throttle_body = throttle_body.expect("expected normal tier to hit limit");
    assert!(
        throttle_body["message"]
            .as_str()
            .unwrap()
            .contains("Rate limit exceeded")
    );
    assert_eq!(throttle_body["error_code"], "FORBIDDEN");
}

#[tokio::test]
async fn enhanced_tier_allows_higher_burst_with_valid_api_key() {
    let app = create_test_app_with_rate_limit(None);

    for _ in 0..40 {
        let (status, body) = send_json(
            &app,
            get_with_headers("/", &[(API_KEY_HEADER, TEST_MASTER_API_KEY)]),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], EXPECTED_ROOT_MESSAGE);
    }
}

#[tokio::test]
async fn health_endpoint_is_not_rate_limited() {
    let app = create_test_app_with_rate_limit(None);

    let mut validated_payload_contract = false;

    for _ in 0..80 {
        let (status, body) = send_json(&app, get_with_headers("/health", &[])).await;
        assert_eq!(status, StatusCode::OK);

        if !validated_payload_contract {
            assert_eq!(body["status"], "ok");
            validated_payload_contract = true;
        }
    }

    assert!(validated_payload_contract);
}
