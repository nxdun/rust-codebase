use crate::{
    config::AppConfig,
    middleware::{
        cors::build_cors,
        rate_limit::{RateLimiters, enforce_tiered_rate_limit, log_rate_limit_mode},
    },
    routes,
    services::ytdlp::YtdlpManager,
    state::AppState,
    telemetry,
};
use axum::{middleware, serve};
use dotenvy::dotenv;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tracing::{error, info};

/// Application entry point.
pub async fn run() {
    // 1. Load environment variables from .env file
    dotenv().ok();

    // 2. Initialize structured logging and environment-based log filtering
    telemetry::init_tracing();
    tracing::info!("nadzu app::run() starting");

    // 3. Configure request/response tracing middleware
    let trace_layer = telemetry::build_trace_layer();

    // 4. Load application config and build shared app state
    let config = match AppConfig::from_env() {
        Ok(cfg) => Arc::new(cfg),
        Err(err) => {
            error!("Failed to load configuration: {err}");
            std::process::exit(1);
        }
    };
    let ytdlp_manager = Arc::new(YtdlpManager::new(config.clone()));
    let rate_limiters = Arc::new(RateLimiters::new());
    log_rate_limit_mode(&config);
    let http_client = reqwest::Client::new();

    let contributions_service =
        Arc::new(crate::services::contributions::ContributionsService::new(
            http_client.clone(),
            config.github_pat().unwrap_or_default().to_string(),
            config
                .github_username
                .clone()
                .unwrap_or_else(|| "nxdun".to_string()),
            config.github_graphql_url.clone(),
        ));

    let state = AppState {
        config: config.clone(),
        ytdlp_manager,
        rate_limiters,
        http_client,
        contributions_service,
    };

    // 5. Build middleware layers (compression + CORS)
    let compression_layer = CompressionLayer::new();
    let cors_layer = build_cors(&config);

    // 6. Compose router, state, and middleware stack (including rate limiter)
    let app = routes::create_router(state.clone())
        .merge(telemetry::setup_metrics_router())
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            enforce_tiered_rate_limit,
        ))
        .layer(trace_layer)
        .layer(cors_layer)
        .layer(compression_layer);

    // 7. Bind TCP listener and log startup info
    let addr = config.addr();
    info!(
        "{} listening on http://{} in {} mode",
        config.name, addr, config.env
    );

    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            // 8. Start HTTP server with graceful shutdown handling.
            if let Err(err) = serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal())
            .await
            {
                error!("server error: {err}");
            }
        }
        Err(err) => {
            error!("Failed to bind to {addr}: {err}");
        }
    }
}

async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        error!("failed to listen for CTRL+C: {err}");
    }
    info!("Initiating graceful shutdown");
}
