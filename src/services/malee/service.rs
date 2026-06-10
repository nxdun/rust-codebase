use reqwest::Client;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::llm::client::{GroqClient, LlmClient};
use crate::services::malee::session_store::SessionStore;

use crate::services::malee::llm::prompt::PromptBuilder;

pub struct MaleeService {
    pub session_store: Arc<SessionStore>,
    pub connector: MaleeConnector,
    pub llm: Box<dyn LlmClient>,
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
    pub fn new(config: &AppConfig, http_client: Client) -> Self {
        tracing::info!("Initializing MaleeService with Groq LLM and MCP Connector");
        let connector = MaleeConnector::new(
            http_client.clone(),
            config.malee_connector_url.clone(),
            config.malee_connector_timeout_ms,
        );

        let llm = Box::new(GroqClient::new(
            http_client,
            config.malee_llm_api_key.clone(),
            config.malee_llm_base_url.clone(),
            config.malee_llm_model.clone(),
            config.malee_llm_fallback_model.clone(),
        ));

        let session_store = SessionStore::new();
        let prompt_builder = PromptBuilder::new();

        Self {
            session_store,
            connector,
            llm,
            prompt_builder,
            config: config.clone(),
        }
    }
}
