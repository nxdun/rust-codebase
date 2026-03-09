use axum::{
    Json,
    extract::{Path, Request, State},
    http::StatusCode,
    http::{HeaderValue, header::CONTENT_DISPOSITION},
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
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let job = state.ytdlp_manager.get_job(&id).await.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "Job not found" })),
    ))?;
    // Prefer video/audio files (avoid serving thumbnails or sidecar files)
    let filename = job
        .files
        .as_ref()
        .and_then(|files| {
            let exts = [
                "mp4", "mkv", "webm", "mov", "mp3", "m4a", "opus", "wav", "flac", "aac",
            ];
            files
                .iter()
                .find(|f| {
                    f.rsplit('.')
                        .next()
                        .map(|ext| exts.contains(&ext))
                        .unwrap_or(false)
                })
                .cloned()
        })
        .ok_or((
            StatusCode::CONFLICT,
            Json(json!({ "error": "No downloadable file yet for this job" })),
        ))?;

    let file_path = PathBuf::from(&job.output_dir).join(&filename);

    if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "File not found" })),
        ));
    }

    match ServeFile::new(file_path).oneshot(req).await {
        Ok(res) => {
            let mut response = res.into_response();
            if let Ok(hv) = HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            {
                response.headers_mut().insert(CONTENT_DISPOSITION, hv);
            }
            Ok(response)
        }
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to serve file: {}", err) })),
        )),
    }
}
