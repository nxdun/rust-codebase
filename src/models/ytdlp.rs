use serde::{Deserialize, Serialize};

/// Internal domain model representing a download job.
/// This includes internal state like `output_dir` and `format_flag` which are not exposed to clients.
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum YtdlpJobStatus {
    Queued,
    Running,
    Finished,
    Failed,
}
