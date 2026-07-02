use crate::state::AppState;
use axum::{Router, routing::get};
use metrics_exporter_prometheus::PrometheusBuilder;
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes structured logging and environment-based log filtering.
/// Supports concurrent logging to Console and File.
pub fn init_tracing() {
    // Force "nadzu" to be included in logs if not specified in RUST_LOG
    let mut env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Add directives safely without unwrapping inside the filter chain
    if let Ok(dir) = "nadzu=debug".parse() {
        env_filter = env_filter.add_directive(dir);
    }
    if let Ok(dir) = "tower_http=info".parse() {
        env_filter = env_filter.add_directive(dir);
    }

    // File appender for persistent logs
    let file_appender = tracing_appender::rolling::daily("logs", "nadzu.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    // We must leak the guard to keep the background worker alive for the process duration
    Box::leak(Box::new(guard));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true);

    let file_layer = fmt::layer()
        .with_ansi(false) // No colors in file logs
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_writer(non_blocking_file);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(file_layer)
        .init();

    tracing::info!("Telemetry system initialized: Console + File sinks active");
}

pub type AppTraceLayer = TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
>;

/// Configures request/response tracing middleware.
#[must_use]
pub fn build_trace_layer() -> AppTraceLayer {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
}

/// Initializes the Prometheus metrics registry and returns the router.
pub fn setup_metrics_router() -> Router<AppState> {
    tracing::info!("Initializing Prometheus metrics recorder");
    let builder = PrometheusBuilder::new();

    // Install the global recorder
    #[allow(clippy::expect_used)]
    let handle = builder
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    // Route that Prometheus will scrape
    Router::new().route("/metrics", get(move || std::future::ready(handle.render())))
}
