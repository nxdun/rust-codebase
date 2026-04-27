use crate::{
    config::AppConfig,
    middleware::{
        cors::build_cors,
        rate_limit::{RateLimiters, enforce_tiered_rate_limit, log_rate_limit_mode},
    },
    routes,
    services::ytdlp::YtdlpManager,
    state::AppState,
};
use axum::{middleware, serve};
use dotenvy::dotenv;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application entry point.
pub async fn run() {
    // 1. Load environment variables from .env file
    dotenv().ok();

    // 2. Initialize structured logging and environment-based log filtering
    #[allow(clippy::expect_used)]
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into())
        .add_directive(
            "tower_http=info"
                .parse()
                .expect("static directive should parse"),
        );

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 3. Configure request/response tracing middleware
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

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
