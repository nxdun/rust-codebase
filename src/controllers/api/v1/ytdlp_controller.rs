use axum::{
    Json,
    extract::{Path, Request, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeFile;

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

pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    req: Request,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let job = state
        .ytdlp_manager
        .get_job(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Job not found".to_string()))?;

    let filename = job.files.as_ref().and_then(|files| files.first()).ok_or((
        StatusCode::CONFLICT,
        "No downloadable file yet for this job".to_string(),
    ))?;

    let file_path = PathBuf::from(&job.output_dir).join(filename);

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    match ServeFile::new(file_path).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serve file: {}", err),
        )),
    }
}
