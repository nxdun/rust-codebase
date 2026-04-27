use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use validator::Validate;

use super::ytdlp::{YtdlpJob, YtdlpJobStatus};

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct YtdlpDownloadRequest {
    #[validate(url(message = "url must be a valid URL"))]
    pub url: String,
    pub quality: Option<String>,
    pub format: Option<String>,
    pub folder: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct YtdlpEnqueueResponse {
    pub status: Cow<'static, str>,
    pub message: Cow<'static, str>,
    pub job: YtdlpJobResponse,
}

#[derive(Debug, Serialize, Clone)]
pub struct YtdlpListResponse {
    pub jobs: Vec<YtdlpJobResponse>,
}

#[derive(Debug, Serialize, Clone)]
pub struct YtdlpJobResponse {
    pub id: String,
    pub url: String,
    pub status: YtdlpJobStatus,
    pub started_at_unix: Option<u64>,
    pub finished_at_unix: Option<u64>,
    pub progress_percent: Option<f32>,
    pub progress_total: Option<String>,
    pub progress_speed: Option<String>,
    pub progress_eta: Option<String>,
    pub progress_message: Option<String>,
    pub updated_at_unix: Option<u64>,
    pub files: Option<Vec<String>>,
    pub error: Option<String>,
}

impl From<YtdlpJob> for YtdlpJobResponse {
    fn from(job: YtdlpJob) -> Self {
        Self {
            id: job.id,
            url: job.url,
            status: job.status,
            started_at_unix: job.started_at_unix,
            finished_at_unix: job.finished_at_unix,
            progress_percent: job.progress_percent,
            progress_total: job.progress_total,
            progress_speed: job.progress_speed,
            progress_eta: job.progress_eta,
            progress_message: job.progress_message,
            updated_at_unix: job.updated_at_unix,
            files: job.files,
            error: job.error,
        }
    }
}
