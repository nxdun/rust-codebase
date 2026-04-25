use crate::common::{TEST_MASTER_API_KEY, create_test_state_with_options, get, send_json};
use axum::http::StatusCode;
use nadzu::models::contributions_model::{
    ContributionMeta, ContributionRange, ContributionSummary, ContributionsResponse,
};
use nadzu::routes::create_router;
use nadzu::services::contributions::ContributionsService;
use std::sync::Arc;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

fn mock_contributions_response(username: &str) -> ContributionsResponse {
    ContributionsResponse {
        username: username.to_string(),
        range: ContributionRange {
            from: "2023-01-01".into(),
            to: "2023-01-02".into(),
            timezone: "UTC".into(),
        },
        summary: ContributionSummary {
            total_contributions: 10,
            total_weeks: 1,
            max_daily_count: 5,
        },
        legend: vec![],
        months: vec![],
        cells: vec![],
        meta: ContributionMeta {
            provider: "github".into(),
            cached: false,
            cache_ttl_seconds: 86400,
            fetched_at: "2023-01-01T00:00:00Z".into(),
            schema_version: 1,
        },
    }
}

#[tokio::test]
async fn get_contributions_returns_seeded_cache() {
    let state = create_test_state_with_options(None, None);
    let mock_resp = mock_contributions_response("nxdun");

    // Seed the cache so it doesn't hit the network
    state
        .contributions_service
        .seed_cache("nxdun", mock_resp, 3600);

    let app = create_router(state.clone()).with_state(state);

    let (status, body) = send_json(&app, get("/api/v1/contributions")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "nxdun");
    assert_eq!(body["meta"]["cached"], true);
}

#[tokio::test]
async fn get_contributions_hits_mock_server_when_cache_empty() {
    let mock_server = MockServer::start().await;

    // Create state pointing to mock server
    let config = nadzu::config::AppConfig {
        name: "test".into(),
        env: "test".into(),
        host: "127.0.0.1".into(),
        port: 8080,
        allowed_origins: None,
        download_dir: "downloads".into(),
        ytdlp_path: "yt-dlp".into(),
        ytdlp_external_downloader: None,
        ytdlp_external_downloader_args: None,
        max_concurrent_downloads: 3,
        captcha_secret_key: None,
        master_api_key: TEST_MASTER_API_KEY.into(),
        github_pat: Some("fake_pat".into()),
        github_username: Some("nxdun".into()),
        github_graphql_url: mock_server.uri(),
    };

    let http_client = reqwest::Client::new();
    let contributions_service = Arc::new(ContributionsService::new(
        http_client.clone(),
        config.github_pat.clone().unwrap(),
        config.github_username.clone().unwrap(),
        config.github_graphql_url.clone(),
    ));

    let state = nadzu::state::AppState {
        config: Arc::new(config.clone()),
        ytdlp_manager: Arc::new(nadzu::services::ytdlp::YtdlpManager::new(Arc::new(config))),
        rate_limiters: Arc::new(nadzu::middleware::rate_limit::RateLimiters::new()),
        http_client,
        contributions_service,
    };

    // Mock GitHub GraphQL response
    let github_response = serde_json::json!({
        "data": {
            "user": {
                "contributionsCollection": {
                    "contributionCalendar": {
                        "totalContributions": 100,
                        "weeks": []
                    }
                }
            }
        }
    });

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(github_response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let app = create_router(state.clone()).with_state(state);
    let (status, body) = send_json(&app, get("/api/v1/contributions")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], "nxdun");
    assert_eq!(body["summary"]["totalContributions"], 100);
    assert_eq!(body["meta"]["cached"], false);
}

#[tokio::test]
async fn get_contributions_rejects_non_default_user() {
    let state = create_test_state_with_options(None, None);
    let app = create_router(state.clone()).with_state(state);

    let (status, body) = send_json(&app, get("/api/v1/contributions?username=someotheruser")).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error_code"], "VALIDATION_ERROR");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("Only the default username is allowed")
    );
}
