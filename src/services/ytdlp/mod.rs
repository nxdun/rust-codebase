use crate::{
    config::AppConfig,
    error::AppError,
    models::{
        ytdlp::{YtdlpJob, YtdlpJobStatus},
        ytdlp_dto::YtdlpDownloadRequest,
    },
};
use dashmap::DashMap;
use regex::Regex;
use std::{
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::{Arc, OnceLock},
    time::SystemTime,
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::{Semaphore, broadcast},
};
use tracing::{debug, error, info};

static PROGRESS_RE: OnceLock<Regex> = OnceLock::new();
static PEAK_RE: OnceLock<Regex> = OnceLock::new();
static QUEUED_RE: OnceLock<Regex> = OnceLock::new();
static FIRST_PCT_RE: OnceLock<Regex> = OnceLock::new();

const MAX_CAPTURED_OUTPUT_BYTES: usize = 8_000;
const YTDLP_TIMEOUT_SECS: u64 = 7_200;
const ARIA2_DOWNLOADER: &str = "aria2c";
const DEFAULT_ARIA2_DOWNLOADER_ARGS: &str =
    "aria2c:-x16 -j16 -s16 -k1M --file-allocation=none --summary-interval=0";

#[derive(Debug, Default)]
struct ParsedProgress {
    percent: Option<f32>,
    total: Option<String>,
    speed: Option<String>,
    eta: Option<String>,
}

/// Manager for handling yt-dlp download jobs.
/// Manages a pool of concurrent downloads using semaphores and tracks job state in a concurrent map.
#[derive(Clone, Debug)]
pub struct YtdlpManager {
    cfg: Arc<AppConfig>,
    jobs: Arc<DashMap<String, YtdlpJob>>,
    semaphore: Arc<Semaphore>,
    job_counter: Arc<std::sync::atomic::AtomicU64>,
    progress_tx: broadcast::Sender<String>,
}

impl YtdlpManager {
    /// Creates a new instance of `YtdlpManager` and starts the background cleanup task.
    #[must_use]
    pub fn new(cfg: Arc<AppConfig>) -> Self {
        let (progress_tx, _) = broadcast::channel(1024);
        let manager = Self {
            semaphore: Arc::new(Semaphore::new(cfg.max_concurrent_downloads)),
            cfg,
            jobs: Arc::new(DashMap::new()),
            job_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            progress_tx,
        };

        let jobs_weak = Arc::downgrade(&manager.jobs);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_mins(10));
            loop {
                interval.tick().await;

                if let Some(jobs) = jobs_weak.upgrade() {
                    let now = now_unix();
                    let retention_period = 3600;

                    jobs.retain(|_, job| {
                        if let Some(finished_at) = job.finished_at_unix {
                            now.saturating_sub(finished_at) < retention_period
                        } else if let Some(started_at) = job.started_at_unix {
                            now.saturating_sub(started_at) < retention_period * 2
                        } else {
                            let parts: Vec<&str> = job.id.split('-').collect();
                            if parts.len() >= 2
                                && let Ok(created_at) = parts[1].parse::<u64>()
                            {
                                return now.saturating_sub(created_at) < retention_period * 2;
                            }
                            true
                        }
                    });
                } else {
                    info!("YtdlpManager dropped, stopping cleanup task");
                    break;
                }
            }
        });

        manager
    }

    /// Enqueues a download job and returns its initial state.
    /// Spawns an asynchronous task to perform the actual download.
    pub fn enqueue_download(&self, payload: YtdlpDownloadRequest) -> YtdlpJob {
        let id = self.next_id();
        let quality = payload.quality.as_deref().unwrap_or("best").to_string();
        let format = payload.format.as_deref().unwrap_or("any").to_string();
        let (format_flag, sort_flag) = resolve_format_selector(&format, &quality);

        let output_dir_res = self.resolve_output_dir(payload.folder.as_deref());
        let output_dir = output_dir_res
            .as_ref()
            .map_or_else(|_| self.cfg.download_dir.clone(), Clone::clone);

        let job = YtdlpJob {
            id: id.clone(),
            url: payload.url.clone(),
            status: YtdlpJobStatus::Queued,
            output_dir: output_dir.clone(),
            format_flag: format_flag.clone(),
            sort_flag: sort_flag.clone(),
            started_at_unix: None,
            finished_at_unix: None,
            progress_percent: None,
            progress_total: None,
            progress_speed: None,
            progress_eta: None,
            progress_message: None,
            updated_at_unix: Some(now_unix()),
            files: None,
            error: None,
        };

        self.jobs.insert(id.clone(), job.clone());

        if let Err(error) = output_dir_res {
            self.update_job(&id, |job| {
                job.status = YtdlpJobStatus::Failed;
                job.error = Some(error.to_string());
                job.finished_at_unix = Some(now_unix());
            });

            #[allow(clippy::expect_used)]
            return self
                .jobs
                .get(&id)
                .map(|e| e.value().clone())
                .expect("job should exist");
        }

        let manager = self.clone();
        tokio::spawn(async move {
            manager
                .run_job(id, payload, output_dir, format_flag, sort_flag)
                .await;
        });

        job
    }

    /// Retrieves a job by ID from the concurrent state map.
    pub fn get_job(&self, id: &str) -> Option<YtdlpJob> {
        self.jobs.get(id).map(|entry| entry.value().clone())
    }

    /// Returns a list of all currently tracked download jobs.
    pub fn list_jobs(&self) -> Vec<YtdlpJob> {
        self.jobs
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Subscribes to the job progress broadcast channel.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.progress_tx.subscribe()
    }

    fn next_id(&self) -> String {
        let ts = now_unix();
        let counter = self
            .job_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("ytdlp-{ts}-{counter}")
    }

    fn resolve_output_dir(&self, folder: Option<&str>) -> Result<String, AppError> {
        let mut dir = PathBuf::from(&self.cfg.download_dir);
        if let Some(folder_str) = folder.filter(|f| !f.is_empty()) {
            let folder_path = Path::new(folder_str);

            if folder_path.is_absolute() {
                return Err(AppError::Validation(
                    "folder must be a relative safe path".to_string(),
                ));
            }

            for component in folder_path.components() {
                if matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                ) {
                    return Err(AppError::Validation(
                        "folder must be a relative safe path".to_string(),
                    ));
                }
            }
            dir.push(folder_path);
        }
        Ok(dir.to_string_lossy().to_string())
    }

    #[allow(clippy::too_many_lines)]
    async fn run_job(
        &self,
        id: String,
        payload: YtdlpDownloadRequest,
        output_dir: String,
        format_flag: String,
        sort_flag: Option<String>,
    ) {
        #[allow(clippy::expect_used)]
        let _permit = self.semaphore.acquire().await.expect("semaphore closed");

        self.mark_job_started(&id);

        let temp_dir = PathBuf::from(&output_dir).join("tmp").join(&id);
        if let Err(err) = fs::create_dir_all(&temp_dir).await {
            self.mark_job_failed(&id, format!("failed to create temp directory: {err}"));
            return;
        }

        let mut cmd = Command::new(&self.cfg.ytdlp_path);
        cmd.kill_on_drop(true);

        cmd.arg("--newline")
            .arg("--no-warnings")
            .arg("--ignore-errors")
            .arg("--concurrent-fragments")
            .arg("8")
            .arg("--buffer-size")
            .arg("16K")
            .arg("-f")
            .arg(&format_flag);

        if let Some(sort_str) = sort_flag {
            cmd.arg("-S").arg(&sort_str);
        }

        if let Some(format_str) = payload.format.as_deref()
            && ["m4a", "mp3", "opus", "wav", "flac"].contains(&format_str)
        {
            cmd.arg("--extract-audio")
                .arg("--audio-format")
                .arg(format_str);
        }

        cmd.arg("-P")
            .arg(&temp_dir)
            .arg("-o")
            .arg(format!("{id}.%(ext)s"))
            .arg(payload.url.clone());

        let downloader = self
            .cfg
            .ytdlp_external_downloader
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(ARIA2_DOWNLOADER);

        let default_args = if downloader == ARIA2_DOWNLOADER {
            DEFAULT_ARIA2_DOWNLOADER_ARGS
        } else {
            ""
        };

        let raw_args = self
            .cfg
            .ytdlp_external_downloader_args
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(default_args);

        cmd.arg("--downloader").arg(downloader);

        if !raw_args.is_empty() {
            let final_args = if raw_args.contains(':') {
                raw_args.to_string()
            } else {
                format!("{downloader}:{raw_args}")
            };
            cmd.arg("--downloader-args").arg(final_args);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        info!("starting ytdlp job id={id} url={}", payload.url);

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(err) => {
                self.mark_job_failed(&id, format!("failed to spawn yt-dlp: {err}"));
                error!("failed ytdlp job id={id}: {err}");
                let _ = fs::remove_dir_all(&temp_dir).await;
                return;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_task = spawn_line_collector(stdout, self.clone(), id.clone());
        let stderr_task = spawn_line_collector(stderr, self.clone(), id.clone());

        let timeout_duration = tokio::time::Duration::from_secs(YTDLP_TIMEOUT_SECS);
        let wait_result = tokio::time::timeout(timeout_duration, child.wait()).await;

        if wait_result.is_err() {
            if let Err(e) = child.kill().await {
                error!("Failed to kill timed-out yt-dlp process id={id}: {e}");
            }
            stdout_task.abort();
            stderr_task.abort();
        }

        let stdout_output = stdout_task.await.unwrap_or_else(|_| String::new());
        let stderr_output = stderr_task.await.unwrap_or_else(|_| String::new());
        let combined_output = combine_outputs(stdout_output, stderr_output);

        match wait_result {
            Ok(Ok(status)) if status.success() => {
                let temp_dir_str = temp_dir.to_string_lossy();
                let files = collect_downloaded_files(&temp_dir_str, &id).await;

                if let Err(err) = fs::create_dir_all(&output_dir).await {
                    self.mark_job_failed(
                        &id,
                        format!("failed to create final output directory: {err}"),
                    );
                } else {
                    let mut moved_files = Vec::with_capacity(files.len());
                    for file in files {
                        let from = temp_dir.join(&file);
                        let to = PathBuf::from(&output_dir).join(&file);
                        if fs::rename(&from, &to).await.is_ok() {
                            moved_files.push(file);
                        }
                    }
                    self.mark_job_finished(&id, moved_files);
                    info!("finished ytdlp job id={id}");
                }
                let _ = fs::remove_dir_all(&temp_dir).await;
            }
            Ok(Ok(status)) => {
                let error_message = truncate_message(&combined_output, 2_000);
                self.mark_job_failed(&id, format!("yt-dlp failed ({status}): {error_message}"));
                Self::cleanup_failed_files(&temp_dir.to_string_lossy(), &id, false).await;
            }
            Ok(Err(err)) => {
                self.mark_job_failed(&id, format!("failed to wait for yt-dlp: {err}"));
                error!("yt-dlp process error for job id={id}: {err}");
                Self::cleanup_failed_files(&temp_dir.to_string_lossy(), &id, false).await;
            }
            Err(_) => {
                self.mark_job_failed(&id, format!("yt-dlp timed out after {YTDLP_TIMEOUT_SECS}s"));
                error!("job timed out id={id}");
                Self::cleanup_failed_files(&temp_dir.to_string_lossy(), &id, false).await;
            }
        }
    }

    async fn cleanup_failed_files(temp_dir: &str, _id: &str, _is_base_dir: bool) {
        let _ = fs::remove_dir_all(temp_dir).await;
    }

    fn update_job<F>(&self, id: &str, update_fn: F)
    where
        F: FnOnce(&mut YtdlpJob),
    {
        if let Some(mut job) = self.jobs.get_mut(id) {
            update_fn(job.value_mut());
            let _ = self.progress_tx.send(id.to_string());
        }
    }

    fn mark_job_started(&self, id: &str) {
        self.update_job(id, |job| {
            job.status = YtdlpJobStatus::Running;
            job.started_at_unix = Some(now_unix());
            job.updated_at_unix = Some(now_unix());
        });
    }

    fn mark_job_failed(&self, id: &str, error_message: String) {
        self.update_job(id, |job| {
            job.status = YtdlpJobStatus::Failed;
            job.error = Some(error_message);
            job.finished_at_unix = Some(now_unix());
            job.updated_at_unix = Some(now_unix());
        });
    }

    fn mark_job_finished(&self, id: &str, files: Vec<String>) {
        self.update_job(id, |job| {
            job.status = YtdlpJobStatus::Finished;
            job.progress_percent = Some(100.0);
            job.files = Some(files);
            job.finished_at_unix = Some(now_unix());
            job.updated_at_unix = Some(now_unix());
        });
    }

    fn ensure_job_running(&self, id: &str) {
        self.update_job(id, |job| {
            if matches!(job.status, YtdlpJobStatus::Queued) {
                job.status = YtdlpJobStatus::Running;
                job.started_at_unix = job.started_at_unix.or_else(|| Some(now_unix()));
            }
        });
    }

    fn apply_progress_line(&self, id: &str, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }

        let sanitized_line = redact_client_progress_line(trimmed);

        debug!(
            "ytdlp progress id={} line={}",
            id,
            truncate_message(trimmed, 500)
        );

        if let Some(parsed) = parse_aria2_progress(trimmed) {
            let aria2_message = format_aria2_progress_message(trimmed, &parsed);
            self.ensure_job_running(id);
            self.update_job(id, |job| {
                if parsed.percent.is_some() {
                    job.progress_percent = parsed.percent;
                }
                if parsed.total.is_some() {
                    job.progress_total = parsed.total;
                }
                if parsed.speed.is_some() {
                    job.progress_speed = parsed.speed;
                }
                if parsed.eta.is_some() {
                    job.progress_eta = parsed.eta;
                }
                job.progress_message = Some(aria2_message);
                job.updated_at_unix = Some(now_unix());
            });
            return;
        }

        if trimmed.starts_with('[') {
            self.update_job(id, |job| {
                job.progress_message = Some(sanitized_line);
                job.updated_at_unix = Some(now_unix());
            });
        }
    }
}

fn redact_client_progress_line(line: &str) -> String {
    if line.contains("Destination") {
        return "[download] Destination: [REDACTED_PATH]".to_string();
    }

    let mut result = String::with_capacity(line.len() + 15);
    for (i, token) in line.split_whitespace().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        if is_sensitive_token(token) {
            result.push_str("[REDACTED_PATH]");
        } else {
            result.push_str(token);
        }
    }
    result
}

fn is_sensitive_token(token: &str) -> bool {
    let cleaned = token.trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == ';');

    if (cleaned.starts_with("${") && cleaned.ends_with('}')) || cleaned.starts_with('$') {
        return true;
    }

    let normalized = cleaned.replace('\\', "/").to_ascii_lowercase();

    normalized.contains("/run/secrets")
        || normalized.contains("/home/app")
        || normalized.starts_with("/home/")
        || normalized.starts_with("/root/")
        || normalized.starts_with("//")
        || is_windows_absolute_path(&normalized)
}

fn is_windows_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

#[allow(clippy::expect_used)]
fn parse_aria2_progress(line: &str) -> Option<ParsedProgress> {
    let re = PROGRESS_RE.get_or_init(|| {
        Regex::new(
            r"\[DL:(?P<total>[^\]]*)\](?:\s+DL:(?P<speed>[^\s]*))?(?:\s+ETA:(?P<eta>[^\s]*))?",
        )
        .expect("invalid progress regex")
    });

    let caps = re.captures(line)?;

    let total = caps
        .name("total")
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty());

    let speed = caps
        .name("speed")
        .map(|m| m.as_str().to_string())
        .filter(|s| !s.is_empty());

    let eta = caps
        .name("eta")
        .map(|m| m.as_str().to_string())
        .filter(|s| !s.is_empty());

    let percent = extract_peak_percent(line).or_else(|| extract_first_percent(line));

    if percent.is_none() && total.is_none() && speed.is_none() && eta.is_none() {
        return None;
    }

    Some(ParsedProgress {
        percent,
        total,
        speed,
        eta,
    })
}

fn format_aria2_progress_message(line: &str, parsed: &ParsedProgress) -> String {
    let active = line.matches("[#").count();
    let queued = extract_queued_fragments(line);
    let peak_percent = extract_peak_percent(line).or(parsed.percent);

    let mut parts = vec!["Downloading fragments".to_string()];

    if let Some(total) = parsed.total.as_deref() {
        parts.push(format!("total {total}"));
    }
    if let Some(percent) = peak_percent {
        parts.push(format!("{percent:.0}%"));
    }
    if active > 0 {
        parts.push(format!("active {active}"));
    }
    if let Some(queued) = queued {
        parts.push(format!("queued +{queued}"));
    }
    if let Some(speed) = parsed.speed.as_deref() {
        parts.push(format!("speed {speed}"));
    }
    if let Some(eta) = parsed.eta.as_deref() {
        parts.push(format!("eta {eta}"));
    }

    parts.join(" | ")
}

#[allow(clippy::expect_used)]
fn extract_queued_fragments(line: &str) -> Option<usize> {
    let re = QUEUED_RE
        .get_or_init(|| Regex::new(r"\(\+(?P<queued>\d+)\)").expect("invalid queued regex"));
    re.captures(line)
        .and_then(|caps| caps.name("queued"))
        .and_then(|m| m.as_str().parse::<usize>().ok())
}

#[allow(clippy::expect_used)]
fn extract_peak_percent(line: &str) -> Option<f32> {
    let re =
        PEAK_RE.get_or_init(|| Regex::new(r"\((?P<pct>[\d.]+)\%\)").expect("invalid peak regex"));
    re.captures_iter(line)
        .filter_map(|caps| {
            caps.name("pct")
                .and_then(|m| m.as_str().parse::<f32>().ok())
        })
        .fold(None, |max, val| Some(max.map_or(val, |m| m.max(val))))
}

#[allow(clippy::expect_used)]
fn extract_first_percent(line: &str) -> Option<f32> {
    let re = FIRST_PCT_RE
        .get_or_init(|| Regex::new(r"(?P<pct>[\d.]+)%").expect("invalid first percent regex"));
    re.captures(line)
        .and_then(|caps| caps.name("pct"))
        .and_then(|m| m.as_str().parse::<f32>().ok())
}

fn spawn_line_collector<R>(
    reader: Option<R>,
    manager: YtdlpManager,
    id: String,
) -> tokio::task::JoinHandle<String>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut captured = String::new();
        if let Some(reader) = reader {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                append_captured_line(&mut captured, &line);
                manager.apply_progress_line(&id, &line);
            }
        }
        captured
    })
}

fn append_captured_line(captured: &mut String, line: &str) {
    if captured.len() >= MAX_CAPTURED_OUTPUT_BYTES {
        return;
    }

    if !captured.is_empty() {
        captured.push('\n');
    }

    let remaining = MAX_CAPTURED_OUTPUT_BYTES.saturating_sub(captured.len());
    if line.len() <= remaining {
        captured.push_str(line);
    } else {
        captured.push_str(&line[..remaining]);
    }
}

fn combine_outputs(stdout_output: String, stderr_output: String) -> String {
    if stderr_output.is_empty() {
        stdout_output
    } else if stdout_output.is_empty() {
        stderr_output
    } else {
        format!("{stderr_output}\n{stdout_output}")
    }
}

async fn collect_downloaded_files(output_dir: &str, id: &str) -> Vec<String> {
    let mut files = Vec::new();
    let id_prefix = format!("{id}.");
    if let Ok(mut entries) = fs::read_dir(output_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(file_name) = entry.file_name().into_string()
                && file_name.starts_with(&id_prefix)
            {
                files.push(file_name);
            }
        }
    }
    files
}

/// Resolves the yt-dlp format selector and sort flags based on requested format and quality.
#[must_use]
pub fn resolve_format_selector(format: &str, quality: &str) -> (String, Option<String>) {
    const AUDIO_FORMATS: [&str; 5] = ["m4a", "mp3", "opus", "wav", "flac"];

    if let Some(custom) = format.strip_prefix("custom:") {
        return (custom.to_string(), None);
    }

    if format == "thumbnail" {
        return ("bestaudio/best".to_string(), None);
    }

    if AUDIO_FORMATS.contains(&format) {
        return ("ba/b".to_string(), Some(format!("aext:{format}")));
    }

    if matches!(format, "mp4" | "any") {
        if quality == "audio" {
            return ("ba/b".to_string(), None);
        }

        let vres = if matches!(quality, "best" | "best_ios" | "worst") {
            String::new()
        } else {
            format!("[height<={quality}]")
        };

        let base_format = format!("bv*{vres}+ba/b");

        let sort = if format == "mp4" {
            Some("res,vcodec:h264,acodec:aac,ext:mp4:m4a".to_string())
        } else {
            None
        };

        return (base_format, sort);
    }

    ("bestvideo+bestaudio/best".to_string(), None)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn truncate_message(message: &str, max: usize) -> String {
    let mut indices = message.char_indices();
    if let Some((idx, _)) = indices.nth(max) {
        format!("{}...", &message[..idx])
    } else {
        message.to_string()
    }
}
