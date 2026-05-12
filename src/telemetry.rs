use crate::state::AppState;
use axum::{Router, routing::get};
use metrics_exporter_prometheus::PrometheusBuilder;
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes structured logging and environment-based log filtering.
pub fn init_tracing() {
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
