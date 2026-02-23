use crate::{
    apply_rate_limiter, config::AppConfig, middleware::cors::build_cors, routes,
    services::ytdlp::YtdlpManager, state::AppState,
};
use axum::serve;
use dotenvy::dotenv;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn run() {
    // 1. Load environment variables from .env file
    dotenv().ok();

    // 2. Initialize structured logging and environment-based log filtering
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("tower_http=info".parse().unwrap())
                .add_directive("axum::rejection=trace".parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 3. Configure request/response tracing middleware
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    // 4. Load application config and build shared app state
    let config = Arc::new(AppConfig::from_env());
    let ytdlp_manager = Arc::new(YtdlpManager::new(config.clone()));
    let http_client = reqwest::Client::new();
    let state = AppState {
        config: config.clone(),
        ytdlp_manager,
        http_client,
    };

    // 5. Build middleware layers (compression + CORS)
    let compression_layer = CompressionLayer::new();
    let cors_layer = build_cors(&config);

    // 6. Compose router, state, and middleware stack (including rate limiter)
    let app = apply_rate_limiter!(routes::create_router(), &config)
        .with_state(state.clone())
        .layer(trace_layer)
        .layer(cors_layer)
        .layer(compression_layer);

    // 7. Bind TCP listener and log startup info
    let addr = config.addr();
    info!(
        "{} listening on http://{} in {} mode",
        config.name, addr, config.env
    );

    let listener = TcpListener::bind(addr).await.unwrap();

    // 8. Start HTTP server with graceful shutdown handling
    if let Err(err) = serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        error!("server error: {}", err);
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    println!();
    info!("graceful shutdown initiated");
}
