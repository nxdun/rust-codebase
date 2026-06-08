use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use metrics::{counter, histogram};
use std::time::Instant;

/// Middleware that records global HTTP traffic metrics.
pub async fn track_http_metrics(req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.extensions().get::<MatchedPath>().map_or_else(
        || req.uri().path().to_string(),
        |matched_path| matched_path.as_str().to_string(),
    );

    let start = Instant::now();
    let response = next.run(req).await;
    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    histogram!(
        "http_request_duration_seconds",
        "method" => method.clone(),
        "path" => path.clone(),
        "status" => status.clone()
    )
    .record(latency);

    counter!(
        "http_requests_total",
        "method" => method,
        "path" => path,
        "status" => status
    )
    .increment(1);

    response
}
