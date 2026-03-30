use axum::{
    Json,
    extract::{Path, Request, State},
    http::StatusCode,
    http::{HeaderValue, header::CONTENT_DISPOSITION},
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde_json::json;
use std::{convert::Infallible, path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::wrappers::ReceiverStream;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::info;

use crate::{
    extractors::validated_json::ValidatedJson,
    models::ytdlp_model::{
        YtdlpDownloadRequest, YtdlpEnqueueResponse, YtdlpJobStatus, YtdlpListResponse,
    },
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

pub async fn stream_download_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
    req: Request,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let request_path = req.uri().path().to_string();
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .or_else(|| req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();
    let initial_job = state.ytdlp_manager.get_job(&id).await;

    if let Some(job) = initial_job.as_ref() {
        info!(
            "sse stream open path={} job_id={} client_ip={} url={}",
            request_path, id, client_ip, job.url
        );
    }

    if initial_job.is_none() {
        info!(
            "sse stream reject path={} job_id={} client_ip={} reason=job_not_found",
            request_path, id, client_ip
        );
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": 404, "message": "job not found" })),
        ));
    }

    let manager = state.ytdlp_manager.clone();
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(16);
    let stream_job_id = id.clone();
    let stream_path = request_path;
    let stream_client_ip = client_ip.clone();

    tokio::spawn(async move {
        let mut last_snapshot = String::new();

        loop {
            match manager.get_job(&stream_job_id).await {
                Some(job) => {
                    let payload = json!({
                        "status": job.status.clone(),
                        "progress_percent": job.progress_percent,
                        "progress_total": job.progress_total.clone(),
                        "progress_speed": job.progress_speed.clone(),
                        "progress_eta": job.progress_eta.clone(),
                        "progress_message": job.progress_message.clone(),
                        "updated_at_unix": job.updated_at_unix
                    });
                    let snapshot = payload.to_string();

                    if snapshot != last_snapshot {
                        if tx
                            .send(Ok(Event::default()
                                .event("progress")
                                .data(snapshot.clone())))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        last_snapshot = snapshot;
                    }

                    if matches!(
                        job.status,
                        YtdlpJobStatus::Finished | YtdlpJobStatus::Failed
                    ) {
                        let _ = tx
                            .send(Ok(Event::default().event("done").data("done")))
                            .await;
                        info!(
                            "sse stream complete path={} job_id={} client_ip={} status={:?}",
                            stream_path, stream_job_id, stream_client_ip, job.status
                        );
                        break;
                    }
                }
                None => {
                    let _ = tx
                        .send(Ok(Event::default()
                            .event("error")
                            .data(r#"{"status":404,"message":"job not found"}"#)))
                        .await;
                    info!(
                        "sse stream ended path={} job_id={} client_ip={} reason=job_not_found",
                        stream_path, stream_job_id, stream_client_ip
                    );
                    break;
                }
            }

            sleep(Duration::from_millis(1500)).await;
        }

        info!(
            "sse stream close path={} job_id={} client_ip={}",
            stream_path, stream_job_id, stream_client_ip
        );
    });

    Ok(Sse::new(ReceiverStream::new(rx)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

pub async fn get_supported_sites() -> (StatusCode, Json<serde_json::Value>) {
    // Read the compiled site lists generated by the docker entrypoint
    let file_path = PathBuf::from("/home/app/sites.txt");
    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => {
            let sites: Vec<&str> = content.lines().filter(|line| !line.is_empty()).collect();
            (
                StatusCode::OK,
                Json(json!({ "status": "ok", "sites": sites })),
            )
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(
                json!({ "error": "Supported sites list not generated yet or missing (requires docker-entrypoint execution)" }),
            ),
        ),
    }
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
