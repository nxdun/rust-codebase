use crate::{config::AppConfig, models::ytdlp_model::*};
use dashmap::DashMap;
use std::{
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::SystemTime,
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::Semaphore,
};
use tracing::{debug, error, info};

const MAX_CAPTURED_OUTPUT_BYTES: usize = 8_000;
const YTDLP_TIMEOUT_SECS: u64 = 7_200;
const ARIA2_DOWNLOADER: &str = "aria2c";
const DEFAULT_ARIA2_DOWNLOADER_ARGS: &str =
    "aria2c:-x16 -j16 -s16 -k1M --file-allocation=none --summary-interval=0";

struct ParsedProgress {
    percent: Option<f32>,
    total: Option<String>,
    speed: Option<String>,
    eta: Option<String>,
}

#[derive(Clone)]
pub struct YtdlpManager {
    cfg: Arc<AppConfig>,
    jobs: Arc<DashMap<String, YtdlpJob>>,
    semaphore: Arc<Semaphore>,
    job_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl YtdlpManager {
    pub fn new(cfg: Arc<AppConfig>) -> Self {
        let manager = Self {
            semaphore: Arc::new(Semaphore::new(cfg.max_concurrent_downloads)),
            cfg,
            jobs: Arc::new(DashMap::new()),
            job_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        };

        // Use a weak reference to avoid keeping the manager alive indefinitely
        let jobs_weak = Arc::downgrade(&manager.jobs);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(600));
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

    pub async fn enqueue_download(&self, payload: YtdlpDownloadRequest) -> YtdlpJob {
        let id = self.next_id();
        let quality = payload.quality.as_deref().unwrap_or("best").to_string();
        let format = payload.format.as_deref().unwrap_or("any").to_string();
        let (format_flag, sort_flag) = resolve_format_selector(&format, &quality);

        let output_dir_res = self.resolve_output_dir(payload.folder.as_deref());
        let output_dir = output_dir_res
            .clone()
            .unwrap_or_else(|_| self.cfg.download_dir.clone());

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
                job.error = Some(error);
                job.finished_at_unix = Some(now_unix());
            });
            return self
                .get_job(&id)
                .await
                .expect("job should exist immediately after insert");
        }

        let manager = self.clone();
        tokio::spawn(async move {
            manager
                .run_job(id, payload, output_dir, format_flag, sort_flag)
                .await;
        });

        job
    }

    pub async fn get_job(&self, id: &str) -> Option<YtdlpJob> {
        self.jobs.get(id).map(|entry| entry.value().clone())
    }

    pub async fn list_jobs(&self) -> Vec<YtdlpJob> {
        self.jobs
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn next_id(&self) -> String {
        let ts = now_unix();
        let counter = self
            .job_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("ytdlp-{}-{}", ts, counter)
    }

    fn resolve_output_dir(&self, folder: Option<&str>) -> Result<String, String> {
        let mut dir = PathBuf::from(&self.cfg.download_dir);
        if let Some(folder_str) = folder.filter(|f| !f.is_empty()) {
            let folder_path = Path::new(folder_str);

            if folder_path.is_absolute() {
                return Err("folder must be a relative safe path".to_string());
            }

            for component in folder_path.components() {
                if matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                ) {
                    return Err("folder must be a relative safe path".to_string());
                }
            }
            dir.push(folder_path);
        }
        Ok(dir.to_string_lossy().to_string())
    }

    async fn run_job(
        &self,
        id: String,
        payload: YtdlpDownloadRequest,
        output_dir: String,
        format_flag: String,
        sort_flag: Option<String>,
    ) {
        let _permit = self.semaphore.acquire().await.expect("semaphore closed");

        self.mark_job_started(&id);

        if let Err(err) = fs::create_dir_all(&output_dir).await {
            self.mark_job_failed(&id, format!("failed to create output directory: {err}"));
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
            .arg(&output_dir)
            .arg("-o")
            .arg(format!("{}.%(ext)s", id))
            .arg(payload.url.clone());

        let aria2_args = self
            .cfg
            .ytdlp_external_downloader_args
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_ARIA2_DOWNLOADER_ARGS);
        cmd.arg("--downloader")
            .arg(ARIA2_DOWNLOADER)
            .arg("--downloader-args")
            .arg(aria2_args);

        let mut job_cookies_file = None;
        if let Some(cookies_file) = self.cfg.ytdlp_cookies_file.as_deref() {
            let temp_cookies = PathBuf::from(&output_dir).join(format!("{id}.cookies.txt"));
            if let Err(err) = fs::copy(cookies_file, &temp_cookies).await {
                self.mark_job_failed(&id, format!("failed to copy cookies file: {err}"));
                return;
            }
            cmd.arg("--cookies").arg(&temp_cookies);
            job_cookies_file = Some(temp_cookies);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        info!("starting ytdlp job id={id} url={}", payload.url);

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(err) => {
                self.mark_job_failed(&id, format!("failed to spawn yt-dlp: {err}"));
                error!("failed ytdlp job id={id}: {err}");
                return;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_task = spawn_line_collector(stdout, self.clone(), id.clone());
        let stderr_task = spawn_line_collector(stderr, self.clone(), id.clone());

        let timeout_duration = tokio::time::Duration::from_secs(YTDLP_TIMEOUT_SECS);
        let wait_result = tokio::time::timeout(timeout_duration, child.wait()).await;
        let stdout_output = stdout_task.await.unwrap_or_else(|_| String::new());
        let stderr_output = stderr_task.await.unwrap_or_else(|_| String::new());
        let combined_output = combine_outputs(stdout_output, stderr_output);

        if let Some(temp_cookies) = job_cookies_file {
            let _ = fs::remove_file(temp_cookies).await;
        }

        match wait_result {
            Ok(Ok(status)) if status.success() => {
                let files = collect_downloaded_files(&output_dir, &id).await;
                self.mark_job_finished(&id, files);
                info!("finished ytdlp job id={id}");
            }
            Ok(Ok(status)) => {
                let error_message = truncate_message(&combined_output, 2_000);
                self.mark_job_failed(
                    &id,
                    format!("yt-dlp failed ({}): {}", status, error_message),
                );
                let is_base_dir = output_dir == self.cfg.download_dir;
                Self::cleanup_failed_files(&output_dir, &id, is_base_dir).await;
            }
            Ok(Err(err)) => {
                self.mark_job_failed(&id, format!("failed to spawn yt-dlp: {err}"));
                error!("failed ytdlp job id={id}: {err}");
                let is_base_dir = output_dir == self.cfg.download_dir;
                Self::cleanup_failed_files(&output_dir, &id, is_base_dir).await;
            }
            Err(_) => {
                if let Err(e) = child.kill().await {
                    error!("Failed to kill timed-out yt-dlp process id={id}: {e}");
                }
                self.mark_job_failed(&id, "yt-dlp process timed out".to_string());
                error!("job timed out id={id}");
                let is_base_dir = output_dir == self.cfg.download_dir;
                Self::cleanup_failed_files(&output_dir, &id, is_base_dir).await;
            }
        }
    }

    async fn cleanup_failed_files(output_dir: &str, id: &str, is_base_dir: bool) {
        let id_prefix = format!("{id}.");
        if let Ok(mut entries) = fs::read_dir(output_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(file_name) = entry.file_name().into_string()
                    && file_name.starts_with(&id_prefix)
                {
                    let _ = fs::remove_file(entry.path()).await;
                }
            }
        }
        if !is_base_dir {
            let _ = fs::remove_dir(output_dir).await;
        }
    }

    fn update_job<F>(&self, id: &str, update_fn: F)
    where
        F: FnOnce(&mut YtdlpJob),
    {
        if let Some(mut job) = self.jobs.get_mut(id) {
            update_fn(job.value_mut());
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
                job.started_at_unix = job.started_at_unix.or(Some(now_unix()));
            }
        });
    }

    fn apply_progress_line(&self, id: &str, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }

        let sanitized_line = redact_client_progress_line(trimmed);

        // Keep raw downloader output at debug level to avoid noisy info logs.
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
                job.progress_message = Some(aria2_message.clone());
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

// Reusable utility for client-safe progress text.
fn redact_client_progress_line(line: &str) -> String {
    if line.contains("Destination") {
        return "[download] Destination: [REDACTED_PATH]".to_string();
    }

    line.split_whitespace()
        .map(|token| {
            if is_sensitive_token(token) {
                "[REDACTED_PATH]".to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn is_sensitive_token(token: &str) -> bool {
    let cleaned = token.trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == ';');

    // Redact common env interpolation patterns.
    if (cleaned.starts_with("${") && cleaned.ends_with('}')) || cleaned.starts_with('$') {
        return true;
    }

    let normalized = cleaned.replace('\\', "/").to_ascii_lowercase();

    // Redact known sensitive server-local path roots and absolute local paths.
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

fn parse_aria2_progress(line: &str) -> Option<ParsedProgress> {
    if !is_aria2_progress_line(line) {
        return None;
    }

    let total = line
        .strip_prefix("[DL:")
        .and_then(|value| value.find(']').map(|idx| value[..idx].trim().to_string()))
        .filter(|value| !value.is_empty())
        .or_else(|| extract_aria2_total_from_fragment(line));

    let percent = extract_peak_percent(line).or_else(|| extract_first_percent(line));
    let speed = extract_after_marker(line, " DL:");
    let eta = extract_after_marker(line, " ETA:");

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

fn is_aria2_progress_line(line: &str) -> bool {
    line.starts_with("[DL:")
        || (line.starts_with('[')
            && (line.contains(" DL:")
                || line.contains(" ETA:")
                || line.contains("CN:")
                || line.contains("%)")))
}

fn extract_aria2_total_from_fragment(line: &str) -> Option<String> {
    line.trim_matches(|c| c == '[' || c == ']')
        .split_whitespace()
        .find_map(|token| {
            let progress = token.split_once('(').map_or(token, |(head, _)| head);
            progress
                .split_once('/')
                .map(|(_, total)| total.trim().trim_matches(']').to_string())
                .filter(|value| !value.is_empty())
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

fn extract_queued_fragments(line: &str) -> Option<usize> {
    let start = line.find("(+")? + 2;
    let tail = &line[start..];
    let end = tail.find(')')?;
    tail[..end].parse::<usize>().ok()
}

fn extract_peak_percent(line: &str) -> Option<f32> {
    let mut max_pct: Option<f32> = None;
    let mut remaining = line;

    while let Some(end_idx) = remaining.find("%)") {
        let upto = &remaining[..end_idx];
        if let Some(start_idx) = upto.rfind('(')
            && let Ok(value) = upto[(start_idx + 1)..].parse::<f32>()
        {
            max_pct = Some(match max_pct {
                Some(current) => current.max(value),
                None => value,
            });
        }
        remaining = &remaining[(end_idx + 3)..];
    }

    max_pct
}

fn extract_first_percent(line: &str) -> Option<f32> {
    let idx = line.find('%')?;
    let prefix = &line[..idx];
    let number = prefix
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    if number.is_empty() {
        None
    } else {
        number.parse::<f32>().ok()
    }
}

fn extract_after_marker(line: &str, marker: &str) -> Option<String> {
    let idx = line.find(marker)?;
    let after = &line[(idx + marker.len())..];
    let value = after
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '/' || *c == ':')
        .collect::<String>();
    if value.is_empty() { None } else { Some(value) }
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

pub fn resolve_format_selector(format: &str, quality: &str) -> (String, Option<String>) {
    const AUDIO_FORMATS: [&str; 5] = ["m4a", "mp3", "opus", "wav", "flac"];

    if let Some(custom) = format.strip_prefix("custom:") {
        return (custom.to_string(), None);
    }

    if format == "thumbnail" {
        return ("bestaudio/best".to_string(), None);
    }

    if AUDIO_FORMATS.contains(&format) {
        return ("ba/b".to_string(), Some(format!("aext:{}", format)));
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
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn truncate_message(message: &str, max: usize) -> String {
    let mut indices = message.char_indices();
    if let Some((idx, _)) = indices.nth(max) {
        format!("{}...", &message[..idx])
    } else {
        message.to_string()
    }
}
