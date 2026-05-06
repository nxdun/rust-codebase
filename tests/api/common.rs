#![allow(unreachable_pub)]
use axum::{
    body::Body,
    http::{Method, Request, Response, StatusCode},
    middleware,
};
use http_body_util::BodyExt;
use nadzu::{
    config::AppConfig,
    middleware::{
        cors::build_cors,
        rate_limit::{RateLimiters, enforce_tiered_rate_limit},
    },
    models::ytdlp_dto::YtdlpDownloadRequest,
    routes::create_router,
    services::{contributions::ContributionsService, ytdlp::YtdlpManager},
    state::AppState,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

pub const HEADER_API_KEY: &str = "x-api-key";
pub const CAPTCHA_TOKEN_HEADER: &str = "x-captcha-token";
pub const CONTENT_TYPE_JSON: &str = "application/json";
pub const TEST_MASTER_API_KEY: &str = "test_master_key";
pub const SAMPLE_YTDLP_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
pub const EXPECTED_ROOT_MESSAGE: &str = "alive";

// State and app builders

pub fn create_test_state(secret_key: Option<&str>) -> AppState {
    create_test_state_with_options(secret_key, None)
}

pub fn create_test_state_with_options(
    secret_key: Option<&str>,
    allowed_origins: Option<&str>,
) -> AppState {
    let config = Arc::new(AppConfig::new(
        "test".into(),
        "test".into(),
        "127.0.0.1".into(),
        8080,
        allowed_origins.map(str::to_string),
        "downloads".into(),
        "yt-dlp".into(),
        None,
        None,
        3,
        secret_key.map(str::to_string),
        TEST_MASTER_API_KEY.into(),
        None,
        None,
        "https://api.github.com/graphql".into(),
    ));

    let ytdlp_manager = Arc::new(YtdlpManager::new(config.clone()));
    let rate_limiters = Arc::new(RateLimiters::new());
    let http_client = reqwest::Client::new();
    let contributions_service = Arc::new(ContributionsService::new(
        http_client.clone(),
        "fake_pat".into(),
        "nxdun".into(),
        config.github_graphql_url.clone(),
    ));

    AppState {
        config,
        ytdlp_manager,
        rate_limiters,
        http_client,
        contributions_service,
    }
}

pub fn create_test_app(secret_key: Option<&str>) -> axum::Router {
    let state = create_test_state(secret_key);
    create_router(state.clone()).with_state(state)
}

pub fn create_test_app_with_rate_limit(secret_key: Option<&str>) -> axum::Router {
    let state = create_test_state(secret_key);
    create_router(state.clone())
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state,
            enforce_tiered_rate_limit,
        ))
}

pub fn create_test_app_with_full_layers(
    secret_key: Option<&str>,
    allowed_origins: Option<&str>,
) -> axum::Router {
    let state = create_test_state_with_options(secret_key, allowed_origins);
    let cors_layer = build_cors(state.config.as_ref());

    create_router(state.clone())
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state,
            enforce_tiered_rate_limit,
        ))
        .layer(cors_layer)
}

// Request builders

#[allow(clippy::expect_used)]
pub fn empty_request(method: Method, uri: &str, headers: &[(&str, &str)]) -> Request<Body> {
    with_headers(Request::builder().method(method).uri(uri), headers)
        .body(Body::empty())
        .expect("failed to build empty request")
}

fn with_headers(
    mut builder: axum::http::request::Builder,
    headers: &[(&str, &str)],
) -> axum::http::request::Builder {
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder
}

pub fn get(uri: &str) -> Request<Body> {
    empty_request(Method::GET, uri, &[])
}

pub fn get_with_headers(uri: &str, headers: &[(&str, &str)]) -> Request<Body> {
    empty_request(Method::GET, uri, headers)
}

pub fn post_json(uri: &str, body: &Value) -> Request<Body> {
    post_json_with_headers(uri, body, &[])
}

pub fn post_json_with_headers(uri: &str, body: &Value, headers: &[(&str, &str)]) -> Request<Body> {
    post_raw_json_with_headers(uri, body.to_string(), headers)
}

#[allow(clippy::expect_used)]
pub fn post_raw_json_with_headers(
    uri: &str,
    raw_body: impl Into<String>,
    headers: &[(&str, &str)],
) -> Request<Body> {
    with_headers(
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("content-type", CONTENT_TYPE_JSON),
        headers,
    )
    .body(Body::from(raw_body.into()))
    .expect("failed to build json post request")
}

// Response helpers

#[allow(clippy::expect_used)]
pub async fn send(app: &axum::Router, request: Request<Body>) -> Response<Body> {
    app.clone()
        .oneshot(request)
        .await
        .expect("request should be handled")
}

pub async fn send_json(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = send(app, request).await;
    let status = response.status();
    let body = get_json_body(response).await;
    (status, body)
}

#[allow(clippy::unwrap_used)]
async fn get_json_body(response: Response<Body>) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

pub fn ytdlp_enqueue_request(url: &str) -> Value {
    json!({
        "url": url,
        "quality": "best",
        "format": "mp4"
    })
}

pub async fn seed_ytdlp_job(state: &AppState, url: &str) -> String {
    let job = state.ytdlp_manager.enqueue_download(YtdlpDownloadRequest {
        url: url.to_string(),
        quality: Some("best".into()),
        format: Some("mp4".into()),
        folder: None,
    });

    job.id
}
