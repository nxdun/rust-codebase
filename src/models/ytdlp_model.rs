use serde::{Deserialize, Serialize};
use validator::Validate;

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
    pub status: &'static str,
    pub message: &'static str,
    pub job: YtdlpJob,
}

#[derive(Debug, Serialize, Clone)]
pub struct YtdlpListResponse {
    pub jobs: Vec<YtdlpJob>,
}

#[derive(Debug, Serialize, Clone)]
pub struct YtdlpJob {
    pub id: String,
    pub url: String,
    pub status: YtdlpJobStatus,
    pub output_dir: String,
    pub format_flag: String,
    pub sort_flag: Option<String>,
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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum YtdlpJobStatus {
    Queued,
    Running,
    Finished,
    Failed,
}
