use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;

use super::client::{
    GenericOpenAiClient, GroqClient, LlmChunk, LlmClient, LlmMessage, OllamaClient, ToolSchema,
};
use crate::error::MaleeError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    Groq,
    Ollama,
    OpenAi,
    Anthropic,
    Google,
    Cerebras,
    Fireworks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmBackendConfig {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: String,
    pub prompt_profile: String,
    pub endpoint: Option<String>,
}

impl LlmBackendConfig {
    pub fn parse_pool(pool_str: &str) -> Result<Vec<Self>, MaleeError> {
        let mut configs = Vec::new();
        for segment in pool_str.split(';') {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }

            // Format: provider:model:api_key[:prompt_profile][@endpoint]
            let (rest, endpoint) = if let Some((r, e)) = segment.split_once('@') {
                (r, Some(e.to_string()))
            } else {
                (segment, None)
            };

            // Custom split to handle :: escape for :
            let mut parts = Vec::new();
            let mut current = String::new();
            let mut chars = rest.chars().peekable();
            while let Some(c) = chars.next() {
                if c == ':' {
                    if chars.peek() == Some(&':') {
                        chars.next(); // Consume second colon
                        current.push(':');
                    } else {
                        parts.push(current);
                        current = String::new();
                    }
                } else {
                    current.push(c);
                }
            }
            parts.push(current);

            if parts.len() < 3 {
                return Err(MaleeError::LlmError(format!(
                    "Invalid LLM pool segment: {segment}"
                )));
            }

            let provider = match parts[0].to_lowercase().as_str() {
                "groq" => LlmProvider::Groq,
                "ollama" => LlmProvider::Ollama,
                "openai" => LlmProvider::OpenAi,
                "anthropic" => LlmProvider::Anthropic,
                "google" => LlmProvider::Google,
                "cerebras" => LlmProvider::Cerebras,
                "fireworks" => LlmProvider::Fireworks,
                _ => {
                    return Err(MaleeError::LlmError(format!(
                        "Unknown provider: {}",
                        parts[0]
                    )));
                }
            };

            let model = parts[1].clone();
            let api_key = parts[2].clone();
            let prompt_profile = parts
                .get(3)
                .cloned()
                .unwrap_or_else(|| "default".to_string());

            configs.push(Self {
                provider,
                model,
                api_key,
                prompt_profile,
                endpoint,
            });
        }

        if configs.is_empty() {
            return Err(MaleeError::LlmError("LLM pool is empty".to_string()));
        }

        Ok(configs)
    }
}

pub struct LlmRouter {
    backends: Vec<Arc<dyn LlmClient>>,
}

impl std::fmt::Debug for LlmRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmRouter")
            .field("backend_count", &self.backends.len())
            .finish()
    }
}

impl LlmRouter {
    pub fn new(http_client: &Client, pool_config: &[LlmBackendConfig]) -> Self {
        let mut backends = Vec::new();
        for config in pool_config {
            let client: Arc<dyn LlmClient> = match config.provider {
                LlmProvider::Groq => Arc::new(GroqClient::new(
                    http_client.clone(),
                    config.api_key.clone(),
                    config
                        .endpoint
                        .clone()
                        .unwrap_or_else(|| "https://api.groq.com/openai/v1".to_string()),
                    config.model.clone(),
                )),
                LlmProvider::Ollama => Arc::new(OllamaClient::new(
                    http_client.clone(),
                    config
                        .endpoint
                        .clone()
                        .unwrap_or_else(|| "http://localhost:11434".to_string()),
                    config.model.clone(),
                )),
                LlmProvider::OpenAi
                | LlmProvider::Cerebras
                | LlmProvider::Fireworks
                | LlmProvider::Google => {
                    let default_endpoint = match config.provider {
                        LlmProvider::Cerebras => "https://api.cerebras.ai/v1".to_string(),
                        LlmProvider::Fireworks => {
                            "https://api.fireworks.ai/inference/v1".to_string()
                        }
                        LlmProvider::Google => {
                            "https://generativelanguage.googleapis.com/v1beta/openai".to_string()
                        }
                        _ => "https://api.openai.com/v1".to_string(),
                    };
                    Arc::new(GenericOpenAiClient::new(
                        http_client.clone(),
                        config.api_key.clone(),
                        config.endpoint.clone().unwrap_or(default_endpoint),
                        config.model.clone(),
                    ))
                }
                LlmProvider::Anthropic => continue, // TODO: Implement Anthropic
            };
            backends.push(client);
        }
        Self { backends }
    }

    pub fn get_backend(&self, index: usize) -> Option<Arc<dyn LlmClient>> {
        self.backends.get(index).cloned()
    }

    pub fn backend_count(&self) -> usize {
        self.backends.len()
    }
}

#[async_trait]
impl LlmClient for LlmRouter {
    async fn stream_chat(
        &self,
        _messages: Vec<LlmMessage>,
        _tools: Vec<ToolSchema>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk, MaleeError>> + Send>>, MaleeError> {
        // This is tricky because LlmClient trait doesn't know about session index or failover state
        // The failover should happen in the agent loop or we need a way to pass the context here.
        // I will implement a custom method for the agent loop to use.
        Err(MaleeError::LlmError(
            "Use LlmRouter::stream_with_failover instead".to_string(),
        ))
    }
}
