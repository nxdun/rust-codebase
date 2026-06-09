use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::MaleeError;

pub const LLM_MODEL: &str = "llama-3.3-70b-versatile";
pub const LLM_FALLBACK_MODEL: &str = "llama-3.1-8b-instant";
pub const LLM_TIMEOUT_MS: u64 = 30000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: LlmFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ToolFunctionSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunctionSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug)]
pub enum LlmChunk {
    Token(String),
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    Done,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream_chat(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<ToolSchema>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk, MaleeError>> + Send>>, MaleeError>;
}

#[derive(Debug)]
pub struct GroqClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    fallback_model: String,
}

impl GroqClient {
    pub const fn new(
        client: Client,
        api_key: String,
        base_url: String,
        model: String,
        fallback_model: String,
    ) -> Self {
        Self {
            client,
            api_key,
            base_url,
            model,
            fallback_model,
        }
    }

    #[allow(clippy::too_many_lines, clippy::unwrap_used)]
    async fn do_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<ToolSchema>,
        use_fallback: bool,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk, MaleeError>> + Send>>, MaleeError> {
        let model = if use_fallback {
            &self.fallback_model
        } else {
            &self.model
        };

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        if !tools.is_empty() {
            body.as_object_mut()
                .unwrap()
                .insert("tools".to_string(), serde_json::to_value(&tools).unwrap());
            body.as_object_mut()
                .unwrap()
                .insert("tool_choice".to_string(), serde_json::json!("auto"));
        }

        let mut response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MaleeError::LlmError(e.to_string()))?;

        if response.status() == 429 && !use_fallback {
            return Box::pin(self.do_stream(messages, tools, true)).await;
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(MaleeError::LlmError(format!("HTTP {status} - {text}")));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut current_tool_id: Option<String> = None;
            let mut current_tool_name = None;
            let mut current_tool_args = String::new();

            while let Ok(Some(chunk)) = response.chunk().await {
                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if line == "data: [DONE]" {
                        let _ = tx.send(Ok(LlmChunk::Done)).await;
                        return;
                    }
                    if let Some(data) = line.strip_prefix("data: ")
                        && let Ok(json) = serde_json::from_str::<serde_json::Value>(data)
                        && let Some(choices) = json.get("choices").and_then(|c| c.as_array())
                    {
                        for choice in choices {
                            if let Some(delta) = choice.get("delta") {
                                if let Some(content) = delta.get("content").and_then(|c| c.as_str())
                                    && !content.is_empty()
                                    && tx
                                        .send(Ok(LlmChunk::Token(content.to_string())))
                                        .await
                                        .is_err()
                                {
                                    return;
                                }

                                if let Some(tool_calls) =
                                    delta.get("tool_calls").and_then(|c| c.as_array())
                                {
                                    for tc in tool_calls {
                                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                            if let Some(ref cid) = current_tool_id {
                                                if cid != id {
                                                    if let (Some(name), Ok(args)) = (
                                                        current_tool_name.clone(),
                                                        serde_json::from_str(&current_tool_args),
                                                    ) && tx
                                                        .send(Ok(LlmChunk::ToolCall {
                                                            id: cid.clone(),
                                                            name,
                                                            arguments: args,
                                                        }))
                                                        .await
                                                        .is_err()
                                                    {
                                                        return;
                                                    }
                                                    current_tool_id = Some(id.to_string());
                                                    current_tool_name = tc
                                                        .get("function")
                                                        .and_then(|f| f.get("name"))
                                                        .and_then(|n| n.as_str())
                                                        .map(ToString::to_string);
                                                    current_tool_args.clear();
                                                }
                                            } else {
                                                current_tool_id = Some(id.to_string());
                                                current_tool_name = tc
                                                    .get("function")
                                                    .and_then(|f| f.get("name"))
                                                    .and_then(|n| n.as_str())
                                                    .map(ToString::to_string);
                                                current_tool_args.clear();
                                            }
                                        }

                                        if let Some(fn_obj) = tc.get("function")
                                            && let Some(args) =
                                                fn_obj.get("arguments").and_then(|a| a.as_str())
                                        {
                                            current_tool_args.push_str(args);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(id) = current_tool_id
                && let (Some(name), Ok(args)) =
                    (current_tool_name, serde_json::from_str(&current_tool_args))
            {
                let _ = tx
                    .send(Ok(LlmChunk::ToolCall {
                        id,
                        name,
                        arguments: args,
                    }))
                    .await;
            }

            let _ = tx.send(Ok(LlmChunk::Done)).await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

#[async_trait]
impl LlmClient for GroqClient {
    async fn stream_chat(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<ToolSchema>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk, MaleeError>> + Send>>, MaleeError> {
        self.do_stream(messages, tools, false).await
    }
}
