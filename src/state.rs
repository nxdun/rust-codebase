use crate::config::AppConfig;
use crate::services::ytdlp::YtdlpManager;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub ytdlp_manager: Arc<YtdlpManager>,
}
