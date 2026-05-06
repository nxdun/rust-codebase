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
use std::{borrow::Cow, convert::Infallible, path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::info;

use crate::{
    error::AppError,
    extractors::json_validator::ValidatedJson,
    models::{
        ytdlp::YtdlpJobStatus,
        ytdlp_dto::{
            YtdlpDownloadRequest, YtdlpEnqueueResponse, YtdlpJobResponse, YtdlpListResponse,
        },
    },
    state::AppState,
};

/// Enqueues a new download job.
pub async fn create_download_job(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<YtdlpDownloadRequest>,
) -> Result<impl IntoResponse, AppError> {
    let job = state.ytdlp_manager.enqueue_download(payload);

    Ok((
        StatusCode::ACCEPTED,
        Json(YtdlpEnqueueResponse {
            status: Cow::Borrowed("accepted"),
            message: Cow::Borrowed("Download enqueued"),
            job: YtdlpJobResponse::from(job),
        }),
    ))
}

/// Retrieves a specific download job by ID.
pub async fn get_download_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let job = state
        .ytdlp_manager
        .get_job(&id)
        .ok_or_else(|| AppError::NotFound(format!("Job {id} not found")))?;

    Ok((StatusCode::OK, Json(YtdlpJobResponse::from(job))))
}

/// Lists all current download jobs.
pub async fn list_download_jobs(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let jobs = state.ytdlp_manager.list_jobs();
    let mut response_jobs = Vec::with_capacity(jobs.len());
    for job in jobs {
        response_jobs.push(YtdlpJobResponse::from(job));
    }
    Ok((
        StatusCode::OK,
        Json(YtdlpListResponse {
            jobs: response_jobs,
        }),
    ))
}

/// Streams progress of a download job via SSE.
#[allow(clippy::too_many_lines)]
pub async fn stream_download_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
    req: Request,
) -> Result<impl IntoResponse, AppError> {
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

    let initial_job = state.ytdlp_manager.get_job(&id).ok_or_else(|| {
        info!(
            "sse stream reject path={} job_id={} client_ip={} reason=job_not_found",
            request_path, id, client_ip
        );
        AppError::NotFound(format!("Job {id} not found"))
    })?;

    info!(
        "sse stream open path={} job_id={} client_ip={} url={}",
        request_path, id, client_ip, initial_job.url
    );

    let manager = state.ytdlp_manager;
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(16);
    let mut broadcast_rx = manager.subscribe();
    let stream_job_id = id;
    let stream_path = request_path;
    let stream_client_ip = client_ip;

    tokio::spawn(async move {
        let mut last_snapshot = String::new();

        // Send initial snapshot
        if let Some(job) = manager.get_job(&stream_job_id) {
            let job_resp = YtdlpJobResponse::from(job);
            let snapshot = serde_json::to_string(&job_resp).unwrap_or_default();
            last_snapshot.clone_from(&snapshot);
            if tx
                .send(Ok(Event::default().event("progress").data(snapshot)))
                .await
                .is_err()
            {
                return;
            }

            if matches!(
                job_resp.status,
                YtdlpJobStatus::Finished | YtdlpJobStatus::Failed
            ) {
                let _ = tx
                    .send(Ok(Event::default().event("done").data("done")))
                    .await;
                return;
            }
        }

        loop {
            match broadcast_rx.recv().await {
                Ok(updated_id) => {
                    if updated_id == stream_job_id {
                        if let Some(job) = manager.get_job(&stream_job_id) {
                            let job_resp = YtdlpJobResponse::from(job);
                            let snapshot = serde_json::to_string(&job_resp).unwrap_or_default();

                            if snapshot != last_snapshot {
                                last_snapshot.clone_from(&snapshot);
                                if tx
                                    .send(Ok(Event::default().event("progress").data(snapshot)))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }

                            if matches!(
                                job_resp.status,
                                YtdlpJobStatus::Finished | YtdlpJobStatus::Failed
                            ) {
                                let _ = tx
                                    .send(Ok(Event::default().event("done").data("done")))
                                    .await;
                                info!(
                                    "sse stream complete path={} job_id={} client_ip={} status={:?}",
                                    stream_path, stream_job_id, stream_client_ip, job_resp.status
                                );
                                break;
                            }
                        } else {
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
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
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

/// Returns a list of supported sites by yt-dlp.
pub async fn get_supported_sites() -> Result<impl IntoResponse, AppError> {
    let file_path = PathBuf::from("/home/app/sites.txt");
    tokio::fs::read_to_string(&file_path).await.map_or_else(
        |_| {
            Err(AppError::ServiceUnavailable(
                "Supported sites list not generated yet or missing".to_string(),
            ))
        },
        |content| {
            let sites: Vec<&str> = content.lines().filter(|line| !line.is_empty()).collect();
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({ "status": "ok", "sites": sites })),
            ))
        },
    )
}

/// Downloads the resulting file of a completed job.
pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    req: Request,
) -> Result<impl IntoResponse, AppError> {
    let job = state
        .ytdlp_manager
        .get_job(&id)
        .ok_or_else(|| AppError::NotFound(format!("Job {id} not found")))?;

    let filename = job
        .files
        .as_ref()
        .and_then(|files| {
            let exts = [
                "mp4", "mkv", "webm", "mov", "mp3", "m4a", "opus", "wav", "flac", "aac",
            ];
            files
                .iter()
                .find(|f| f.rsplit('.').next().is_some_and(|ext| exts.contains(&ext)))
                .cloned()
        })
        .ok_or_else(|| AppError::Conflict("No downloadable file yet for this job".to_string()))?;

    let file_path = PathBuf::from(&job.output_dir).join(&filename);

    if !tokio::fs::try_exists(&file_path).await? {
        return Err(AppError::NotFound(format!("File {filename} not found")));
    }

    let res = ServeFile::new(file_path)
        .oneshot(req)
        .await
        .map_err(|err| AppError::Internal(anyhow::Error::new(err)))?;

    let mut response = res.into_response();
    if let Ok(hv) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        response.headers_mut().insert(CONTENT_DISPOSITION, hv);
    }
    Ok(response)
}
