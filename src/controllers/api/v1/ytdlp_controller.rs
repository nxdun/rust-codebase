use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::json;

use crate::{
    extractors::validated_json::ValidatedJson,
    models::ytdlp_model::{YtdlpDownloadRequest, YtdlpEnqueueResponse, YtdlpListResponse},
    state::AppState,
};

pub async fn create_download_job(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<YtdlpDownloadRequest>,
) -> (StatusCode, Json<YtdlpEnqueueResponse>) {
    let job = state.ytdlp_manager.enqueue_download(payload).await;

    (
        StatusCode::ACCEPTED,
        Json(YtdlpEnqueueResponse {
            status: "accepted",
            message: "Download enqueued",
            job,
        }),
    )
}

pub async fn get_download_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.ytdlp_manager.get_job(&id).await {
        Some(job) => (StatusCode::OK, Json(json!({ "job": job }))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "status": 404, "message": "job not found" })),
        ),
    }
}

pub async fn list_download_jobs(
    State(state): State<AppState>,
) -> (StatusCode, Json<YtdlpListResponse>) {
    let jobs = state.ytdlp_manager.list_jobs().await;
    (StatusCode::OK, Json(YtdlpListResponse { jobs }))
}
