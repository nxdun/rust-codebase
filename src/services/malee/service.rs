use reqwest::Client;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::llm::pool::LlmRouter;
use crate::services::malee::session_store::SessionStore;

use crate::services::malee::llm::prompt::PromptBuilder;

pub struct MaleeService {
    pub session_store: Arc<SessionStore>,
    pub connector: MaleeConnector,
    pub llm_router: LlmRouter,
    pub prompt_builder: PromptBuilder,
    pub config: AppConfig, // To keep config values accessible if needed
}

impl std::fmt::Debug for MaleeService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaleeService").finish_non_exhaustive()
    }
}

impl MaleeService {
    #[tracing::instrument(skip(config, http_client))]
    pub fn new(config: &AppConfig, http_client: &Client) -> Self {
        tracing::info!("Initializing MaleeService with Multi-Provider LLM Pool");
        let connector = MaleeConnector::new(
            http_client.clone(),
            config.malee_connector_url.clone(),
            config.malee_connector_timeout_ms,
        );

        let llm_router = LlmRouter::new(http_client, &config.malee_llm_pool);
        let session_store = SessionStore::new();
        let prompt_builder = PromptBuilder::new();

        Self {
            session_store,
            connector,
            llm_router,
            prompt_builder,
            config: config.clone(),
        }
    }
}
