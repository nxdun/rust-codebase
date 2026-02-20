use crate::{config::AppConfig, models::ytdlp_model::*};
use dashmap::DashMap;
use std::{path::PathBuf, sync::Arc, time::SystemTime};
use tokio::{
    fs,
    process::Command,
    sync::Semaphore,
};
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

        // Spawn cleanup task: remove jobs older than 1 hour (3600s)
        // Use a weak reference to avoid keeping the manager alive indefinitely
        let jobs_weak = Arc::downgrade(&manager.jobs);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(600)); // Run every 10 mins
            loop {
                interval.tick().await;

                // Check if the improved YtdlpManager still exists
                if let Some(jobs) = jobs_weak.upgrade() {
                    let now = now_unix();
                    let retention_period = 3600; 

                    // DashMap defines retain which is efficient for removal
                    jobs.retain(|_, job| {
                        match job.finished_at_unix {
                            Some(finished_at) => now.saturating_sub(finished_at) < retention_period,
                            None => true, // Keep running/queued jobs
                        }
                    });
                } else {
                    // Manager was dropped, stop the background task
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
        let quality = payload.quality.clone().unwrap_or_else(|| "best".to_string());
        let format = payload.format.clone().unwrap_or_else(|| "any".to_string());
        let format_selector = resolve_format_selector(&format, &quality);
        
        // Resolve output directory once here
        let output_dir_res = self.resolve_output_dir(payload.folder.as_deref());
        let output_dir = output_dir_res.clone().unwrap_or_else(|_| self.cfg.download_dir.clone());

        let job = YtdlpJob {
            id: id.clone(),
            url: normalized_url.clone(),
            status: YtdlpJobStatus::Queued,
            output_dir: output_dir.clone(),
            format_selector: format_selector.clone(),
            started_at_unix: None,
            finished_at_unix: None,
            error: None,
        };

        self.jobs.insert(id.clone(), job.clone());

        if let Err(error) = output_dir_res {
            self.update_status(&id, YtdlpJobStatus::Failed, Some(error), None, Some(now_unix()));
            return self
                .get_job(&id)
                .await
                .expect("job should exist immediately after insert");
        }

        let mut payload = payload;
        payload.url = normalized_url;

        let manager = self.clone();
        tokio::spawn(async move {
            // Pass the pre-resolved output_dir to run_job
            manager.run_job(id, payload, output_dir, format_selector).await;
        });

        job
    }

    pub async fn get_job(&self, id: &str) -> Option<YtdlpJob> {
        self.jobs.get(id).map(|entry| entry.value().clone())
    }

    pub async fn list_jobs(&self) -> Vec<YtdlpJob> {
        self.jobs.iter().map(|entry| entry.value().clone()).collect()
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
        if let Some(folder) = folder.filter(|f| !f.is_empty()) {
            if folder.contains("..") || folder.starts_with('/') || folder.starts_with('\\') {
                return Err("folder must be a relative safe path".to_string());
            }
            dir.push(folder);
        }
        Ok(dir.to_string_lossy().to_string())
    }

    async fn run_job(&self, id: String, payload: YtdlpDownloadRequest, output_dir: String, selector: String) {
        let _permit = self.semaphore.acquire().await.expect("semaphore closed");

        self.update_status(&id, YtdlpJobStatus::Running, None, Some(now_unix()), None);

        if let Err(err) = fs::create_dir_all(&output_dir).await {
            self.update_status(
                &id,
                YtdlpJobStatus::Failed,
                Some(format!("failed to create output directory: {err}")),
                None,
                Some(now_unix()),
            );
            return;
        }

        let mut cmd = Command::new(payload.ytdlp_path.as_deref().unwrap_or(&self.cfg.ytdlp_path));

        cmd.arg("--newline")
            .arg("--no-warnings")
            .arg("--ignore-errors")
            .arg("--concurrent-fragments")
            .arg("8")
            .arg("--buffer-size")
            .arg("16K")
            .arg("-f")
            .arg(&selector)
            .arg("-P")
            .arg(&output_dir)
            .arg("-o")
            .arg(self.build_output_template(payload.custom_name_prefix.as_deref()))
            .arg(payload.url.clone());

        if let Some(cookies_file) = self.cfg.ytdlp_cookies_file.as_deref() {
            cmd.arg("--cookies").arg(cookies_file);
        }

        let pot_extractor_args = self
            .cfg
            .ytdlp_pot_provider_url
            .as_deref()
            .map(to_pot_extractor_args);
        let extractor_args = self
            .cfg
            .ytdlp_extractor_args
            .clone()
            .or(pot_extractor_args);

        if let Some(extractor_args) = extractor_args {
            cmd.arg("--extractor-args").arg(extractor_args);
        }

        info!("starting ytdlp job id={id} url={}", payload.url);

        let output = cmd.output().await;
        match output {
            Ok(result) if result.status.success() => {
                self.update_status(&id, YtdlpJobStatus::Finished, None, None, Some(now_unix()));
                info!("finished ytdlp job id={id}");
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let error_message = truncate_message(stderr.as_ref(), 2_000);
                self.update_status(
                    &id,
                    YtdlpJobStatus::Failed,
                    Some(format!("yt-dlp failed ({}): {}", result.status, error_message)),
                    None,
                    Some(now_unix()),
                );
            }
            Err(err) => {
                self.update_status(
                    &id,
                    YtdlpJobStatus::Failed,
                    Some(format!("failed to spawn yt-dlp: {err}")),
                    None,
                    Some(now_unix()),
                );
                error!("failed ytdlp job id={id}: {err}");
            }
        }
    }

    fn update_status(
        &self,
        id: &str,
        status: YtdlpJobStatus,
        error: Option<String>,
        started_at: Option<u64>,
        finished_at: Option<u64>,
    ) {
        if let Some(mut job) = self.jobs.get_mut(id) {
            job.status = status;
            if let Some(err) = error {
                job.error = Some(err);
            }
            if let Some(ts) = started_at {
                job.started_at_unix = Some(ts);
            }
            if let Some(ts) = finished_at {
                job.finished_at_unix = Some(ts);
            }
        }
    }

    fn build_output_template(&self, prefix: Option<&str>) -> String {
        let base = &self.cfg.ytdlp_output_template;
        match prefix.filter(|v| !v.is_empty()) {
            Some(value) => format!("{value}.{base}"),
            None => base.to_string(),
        }
    }
}

fn resolve_format_selector(format: &str, quality: &str) -> String {
    const AUDIO_FORMATS: [&str; 5] = ["m4a", "mp3", "opus", "wav", "flac"];

    if let Some(custom) = format.strip_prefix("custom:") {
        return custom.to_string();
    }

    if format == "thumbnail" {
        return "bestaudio/best".to_string();
    }

    if AUDIO_FORMATS.contains(&format) {
        return format!("bestaudio[ext={format}]/bestaudio/best");
    }

    if matches!(format, "mp4" | "any") {
        if quality == "audio" {
            return "bestaudio/best".to_string();
        }

        let (vfmt, afmt) = if format == "mp4" {
            ("[ext=mp4]", "[ext=m4a]")
        } else {
            ("", "")
        };

        let vres = if matches!(quality, "best" | "best_ios" | "worst") {
            String::new()
        } else {
            format!("[height<={quality}]")
        };

        let vcombo = format!("{vres}{vfmt}");

        if quality == "best_ios" {
            return format!(
                "bestvideo[vcodec~='^((he|a)vc|h26[45])']{vres}+bestaudio[acodec=aac]/bestvideo[vcodec~='^((he|a)vc|h26[45])']{vres}+bestaudio{afmt}/bestvideo{vcombo}+bestaudio{afmt}/best{vcombo}"
            );
        }

        return format!("bestvideo{vcombo}+bestaudio{afmt}/best{vcombo}");
    }

    "bestvideo+bestaudio/best".to_string()
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn truncate_message(message: &str, max: usize) -> String {
    if message.len() <= max {
        return message.to_string();
    }
    let head = &message[..max];
    format!("{head}...")
}

fn to_pot_extractor_args(url: &str) -> String {
    format!("youtube:po_token_provider=bgutil:{url}")
}

fn normalize_youtube_url(url: &str) -> String {
    if let Some(short_id) = extract_shorts_id(url) {
        return format!("https://www.youtube.com/watch?v={short_id}");
    }
    url.to_string()
}

fn extract_shorts_id(url: &str) -> Option<&str> {
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
