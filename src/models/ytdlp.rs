use serde::{Deserialize, Serialize};

/// Internal domain model representing a download job.
/// This includes internal state like `output_dir` and `format_flag` which are not exposed to clients.
#[derive(Debug, Serialize, Clone)]
pub struct YtdlpJob {
    /// The unique identifier of the job.
    pub id: String,
    /// The URL being downloaded.
    pub url: String,
    /// The current status of the job.
    pub status: YtdlpJobStatus,
    /// Internal: The output directory for downloaded files.
    pub output_dir: String,
    /// Internal: The format flag passed to yt-dlp.
    pub format_flag: String,
    /// Internal: The sort flag passed to yt-dlp.
    pub sort_flag: Option<String>,
    /// UNIX timestamp when the job started.
    pub started_at_unix: Option<u64>,
    /// UNIX timestamp when the job finished.
    pub finished_at_unix: Option<u64>,
    /// The current progress percentage.
    pub progress_percent: Option<f32>,
    /// The total size of the download if known.
    pub progress_total: Option<String>,
    /// The current download speed.
    pub progress_speed: Option<String>,
    /// The estimated time of arrival.
    pub progress_eta: Option<String>,
    /// General progress message or extraction details.
    pub progress_message: Option<String>,
    /// UNIX timestamp when the job was last updated.
    pub updated_at_unix: Option<u64>,
    /// The list of files downloaded by the job.
    pub files: Option<Vec<String>>,
    /// Any error message encountered during the job.
    pub error: Option<String>,
}

/// The status of a yt-dlp download job.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum YtdlpJobStatus {
    /// The job is queued and waiting to start.
    Queued,
    /// The job is currently running.
    Running,
    /// The job has finished successfully.
    Finished,
    /// The job failed.
    Failed,
}
