use crate::{config::AppConfig, models::ytdlp_model::*};
use dashmap::DashMap;
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};
use tokio::{fs, process::Command, sync::Semaphore};
use tracing::{error, info};

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
        let normalized_url = normalize_youtube_url(&payload.url);
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
            url: normalized_url.clone(),
            status: YtdlpJobStatus::Queued,
            output_dir: output_dir.clone(),
            format_flag: format_flag.clone(),
            sort_flag: sort_flag.clone(),
            started_at_unix: None,
            finished_at_unix: None,
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

        let mut payload = payload;
        payload.url = normalized_url;

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

        self.update_job(&id, |job| {
            job.status = YtdlpJobStatus::Running;
            job.started_at_unix = Some(now_unix());
        });

        if let Err(err) = fs::create_dir_all(&output_dir).await {
            self.update_job(&id, |job| {
                job.status = YtdlpJobStatus::Failed;
                job.error = Some(format!("failed to create output directory: {err}"));
                job.finished_at_unix = Some(now_unix());
            });
            return;
        }

        let mut cmd = Command::new(&self.cfg.ytdlp_path);
        cmd.kill_on_drop(true);

        cmd.arg("--no-progress")
            .arg("--newline")
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

        if let Some(format_str) = payload.format.as_deref() {
            if ["m4a", "mp3", "opus", "wav", "flac"].contains(&format_str) {
                cmd.arg("--extract-audio")
                    .arg("--audio-format")
                    .arg(format_str);
            }
        }

        cmd.arg("-P")
            .arg(&output_dir)
            .arg("-o")
            .arg(format!("{}.%(ext)s", id))
            .arg(payload.url.clone());

        let mut job_cookies_file = None;
        if let Some(cookies_file) = self.cfg.ytdlp_cookies_file.as_deref() {
            let temp_cookies = PathBuf::from(&output_dir).join(format!("{id}.cookies.txt"));
            if let Err(err) = fs::copy(cookies_file, &temp_cookies).await {
                self.update_job(&id, |job| {
                    job.status = YtdlpJobStatus::Failed;
                    job.error = Some(format!("failed to copy cookies file: {err}"));
                    job.finished_at_unix = Some(now_unix());
                });
                return;
            }
            cmd.arg("--cookies").arg(&temp_cookies);
            job_cookies_file = Some(temp_cookies);
        }

        let extractor_args = self
            .cfg
            .ytdlp_extractor_args
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                self.cfg
                    .ytdlp_pot_provider_url
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(to_pot_extractor_args)
            });

        if let Some(extractor_args) = extractor_args {
            cmd.arg("--extractor-args").arg(extractor_args);
        }

        info!("starting ytdlp job id={id} url={}", payload.url);

        let timeout_duration = tokio::time::Duration::from_secs(7200);

        let output_result = tokio::time::timeout(timeout_duration, cmd.output()).await;

        if let Some(temp_cookies) = job_cookies_file {
            let _ = fs::remove_file(temp_cookies).await;
        }

        match output_result {
            Ok(Ok(result)) if result.status.success() => {
                let mut files = Vec::new();
                if let Ok(mut entries) = fs::read_dir(&output_dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let id_prefix = format!("{id}.");
                        if let Ok(file_name) = entry.file_name().into_string()
                            && file_name.starts_with(&id_prefix)
                        {
                            files.push(file_name);
                        }
                    }
                }
                self.update_job(&id, |job| {
                    job.status = YtdlpJobStatus::Finished;
                    job.files = Some(files);
                    job.finished_at_unix = Some(now_unix());
                });
                info!("finished ytdlp job id={id}");
            }
            Ok(Ok(result)) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let error_message = truncate_message(stderr.as_ref(), 2_000);
                self.update_job(&id, |job| {
                    job.status = YtdlpJobStatus::Failed;
                    job.error = Some(format!(
                        "yt-dlp failed ({}): {}",
                        result.status, error_message
                    ));
                    job.finished_at_unix = Some(now_unix());
                });
                let is_base_dir = output_dir == self.cfg.download_dir;
                Self::cleanup_failed_files(&output_dir, &id, is_base_dir).await;
            }
            Ok(Err(err)) => {
                self.update_job(&id, |job| {
                    job.status = YtdlpJobStatus::Failed;
                    job.error = Some(format!("failed to spawn yt-dlp: {err}"));
                    job.finished_at_unix = Some(now_unix());
                });
                error!("failed ytdlp job id={id}: {err}");
                let is_base_dir = output_dir == self.cfg.download_dir;
                Self::cleanup_failed_files(&output_dir, &id, is_base_dir).await;
            }
            Err(_) => {
                self.update_job(&id, |job| {
                    job.status = YtdlpJobStatus::Failed;
                    job.error = Some("yt-dlp process timed out".to_string());
                    job.finished_at_unix = Some(now_unix());
                });
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

fn to_pot_extractor_args(url: &str) -> String {
    format!("youtube:po_token_provider=bgutil:{url}")
}

pub fn normalize_youtube_url(url: &str) -> String {
    if let Some(short_id) = extract_shorts_id(url) {
        return format!("https://www.youtube.com/watch?v={short_id}");
    }
    url.to_string()
}

pub fn extract_shorts_id(url: &str) -> Option<&str> {
    let marker = "/shorts/";
    let idx = url.find(marker)?;
    let start = idx + marker.len();
    if start >= url.len() {
        return None;
    }

    let tail = &url[start..];
    let end = tail.find(&['?', '&', '/'][..]).unwrap_or(tail.len());
    let video_id = &tail[..end];

    if video_id.is_empty() {
        None
    } else {
        Some(video_id)
    }
}
