use crate::config::AppConfig;
use crate::middleware::rate_limit::RateLimiters;
use crate::services::ytdlp::YtdlpManager;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub ytdlp_manager: Arc<YtdlpManager>,
    pub rate_limiters: Arc<RateLimiters>,
    pub http_client: reqwest::Client,
}
