use reqwest::Client;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::services::malee::connector::client::MaleeConnector;
use crate::services::malee::llm::client::{GroqClient, LlmClient};
use crate::services::malee::session_store::SessionStore;

pub struct MaleeService {
    pub session_store: Arc<SessionStore>,
    pub connector: MaleeConnector,
    pub llm: Box<dyn LlmClient>,
    pub config: AppConfig, // To keep config values accessible if needed
}

impl std::fmt::Debug for MaleeService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaleeService").finish_non_exhaustive()
    }
}

impl MaleeService {
    pub fn new(config: &AppConfig, http_client: Client) -> Self {
        let connector = MaleeConnector::new(
            http_client.clone(),
            config.malee_connector_url.clone(),
            crate::services::malee::connector::client::CONNECTOR_TIMEOUT_MS,
        );

        let llm = Box::new(GroqClient::new(
            http_client,
            config.malee_llm_api_key.clone(),
            config.malee_llm_base_url.clone(),
            crate::services::malee::llm::client::LLM_MODEL.to_string(),
            crate::services::malee::llm::client::LLM_FALLBACK_MODEL.to_string(),
        ));

        let session_store = SessionStore::new();

        Self {
            session_store,
            connector,
            llm,
            config: config.clone(),
        }
    }
}
