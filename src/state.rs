use crate::config::AppConfig;
use crate::middleware::rate_limit::RateLimiters;
use crate::services::contributions::ContributionsService;
use crate::services::ytdlp::YtdlpManager;
use std::sync::Arc;

/// Represents the global application state shared across all routes and services.
#[derive(Clone, Debug)]
pub struct AppState {
    /// Global application configuration.
    pub config: Arc<AppConfig>,
    /// Manager for YT-DLP processes and jobs.
    pub ytdlp_manager: Arc<YtdlpManager>,
    /// Rate limiters for different endpoints.
    pub rate_limiters: Arc<RateLimiters>,
    /// Global HTTP client for making outbound requests.
    pub http_client: reqwest::Client,
    /// Service for handling GitHub contribution data.
    pub contributions_service: Arc<ContributionsService>,
}
