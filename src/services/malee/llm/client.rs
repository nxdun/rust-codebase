use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::MaleeError;

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
}

impl GroqClient {
    pub const fn new(client: Client, api_key: String, base_url: String, model: String) -> Self {
        Self {
            client,
            api_key,
            base_url,
            model,
        }
    }

    #[tracing::instrument(skip(self, messages, tools), fields(model = %self.model, messages_len = messages.len(), tools_len = tools.len()))]
    #[allow(clippy::too_many_lines, clippy::unwrap_used, clippy::type_complexity)]
    async fn do_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<ToolSchema>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk, MaleeError>> + Send>>, MaleeError> {
        let mut body = serde_json::json!({
            "model": &self.model,
            "messages": messages,
            "stream": true,
        });

        if !tools.is_empty()
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert(
                "tools".to_string(),
                serde_json::to_value(&tools).map_err(|e| MaleeError::LlmError(e.to_string()))?,
            );
            obj.insert("tool_choice".to_string(), serde_json::json!("auto"));
        }

        tracing::debug!(
            "Sending request to Groq: {} | Body: {}",
            self.model,
            serde_json::to_string(&body).unwrap_or_default()
        );
        let mut response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("LLM request failed: {}", e);
                MaleeError::LlmError(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!("LLM HTTP error: {} - {}", status, text);
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(MaleeError::LlmError(format!("RATE_LIMIT:{text}")));
            }
            return Err(MaleeError::LlmError(format!("HTTP {status} - {text}")));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut current_tool_id: Option<String> = None;
            let mut current_tool_name = None;
            let mut current_tool_args = String::new();
            let mut buffer = String::new();

            tracing::debug!("Starting LLM stream processing");
            while let Ok(Some(chunk)) = response.chunk().await {
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer.drain(..=line_end);

                    if line.is_empty() {
                        continue;
                    }
                    if line == "data: [DONE]" {
                        tracing::debug!("LLM stream reached [DONE]");
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
                        return;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        match serde_json::from_str::<serde_json::Value>(data) {
                            Ok(json) => {
                                if let Some(err) = json.get("error") {
                                    tracing::error!("LLM stream error field: {}", err);
                                    let _ =
                                        tx.send(Err(MaleeError::LlmError(err.to_string()))).await;
                                    return;
                                }

                                if let Some(choices) =
                                    json.get("choices").and_then(serde_json::Value::as_array)
                                {
                                    for choice in choices {
                                        if let Some(delta) = choice.get("delta") {
                                            if let Some(content) = delta
                                                .get("content")
                                                .and_then(serde_json::Value::as_str)
                                                && !content.is_empty()
                                                && tx
                                                    .send(Ok(LlmChunk::Token(content.to_string())))
                                                    .await
                                                    .is_ok()
                                            {
                                            }

                                            if let Some(tool_calls) = delta
                                                .get("tool_calls")
                                                .and_then(serde_json::Value::as_array)
                                            {
                                                for tc in tool_calls {
                                                    if let Some(id) = tc
                                                        .get("id")
                                                        .and_then(serde_json::Value::as_str)
                                                    {
                                                        if let Some(ref cid) = current_tool_id {
                                                            if cid != id {
                                                                if let (Some(name), Ok(args)) = (
                                                                    current_tool_name.clone(),
                                                                    serde_json::from_str(
                                                                        &current_tool_args,
                                                                    ),
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
                                                                current_tool_id =
                                                                    Some(id.to_string());
                                                                current_tool_name = tc
                                                                    .get("function")
                                                                    .and_then(|f| f.get("name"))
                                                                    .and_then(
                                                                        serde_json::Value::as_str,
                                                                    )
                                                                    .map(ToString::to_string);
                                                                current_tool_args.clear();
                                                            }
                                                        } else {
                                                            current_tool_id = Some(id.to_string());
                                                            current_tool_name = tc
                                                                .get("function")
                                                                .and_then(|f| f.get("name"))
                                                                .and_then(serde_json::Value::as_str)
                                                                .map(ToString::to_string);
                                                            current_tool_args.clear();
                                                        }
                                                    }

                                                    if let Some(fn_obj) = tc.get("function")
                                                        && let Some(args) = fn_obj
                                                            .get("arguments")
                                                            .and_then(serde_json::Value::as_str)
                                                    {
                                                        current_tool_args.push_str(args);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to parse LLM stream JSON: {} | Raw: {}",
                                    e,
                                    data
                                );
                            }
                        }
                    } else {
                        tracing::debug!("Ignoring non-data SSE line: {}", line);
                    }
                }
            }

            tracing::debug!("LLM chunk stream ended (Ok(None))");

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
        self.do_stream(messages, tools).await
    }
}

#[derive(Debug)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub const fn new(client: Client, base_url: String, model: String) -> Self {
        Self {
            client,
            base_url,
            model,
        }
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    #[tracing::instrument(skip(self, messages, tools), fields(model = %self.model))]
    async fn stream_chat(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<ToolSchema>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmChunk, MaleeError>> + Send>>, MaleeError> {
        let mut body = serde_json::json!({
            "model": &self.model,
            "messages": messages,
            "stream": true,
        });

        if !tools.is_empty()
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert(
                "tools".to_string(),
                serde_json::to_value(&tools).map_err(|e| MaleeError::LlmError(e.to_string()))?,
            );
        }

        let mut response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| MaleeError::LlmError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(MaleeError::LlmError(format!(
                "Ollama HTTP {status} - {text}"
            )));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = response.chunk().await {
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer.drain(..=line_end);

                    if line.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<serde_json::Value>(&line) {
                        Ok(json) => {
                            if let Some(done) =
                                json.get("done").and_then(serde_json::Value::as_bool)
                                && done
                            {
                                let _ = tx.send(Ok(LlmChunk::Done)).await;
                                return;
                            }

                            if let Some(message) = json.get("message") {
                                if let Some(content) =
                                    message.get("content").and_then(serde_json::Value::as_str)
                                    && !content.is_empty()
                                    && tx
                                        .send(Ok(LlmChunk::Token(content.to_string())))
                                        .await
                                        .is_ok()
                                {}

                                if let Some(tool_calls) = message
                                    .get("tool_calls")
                                    .and_then(serde_json::Value::as_array)
                                {
                                    for tc in tool_calls {
                                        let name = tc
                                            .get("function")
                                            .and_then(|f| f.get("name"))
                                            .and_then(serde_json::Value::as_str)
                                            .unwrap_or_default()
                                            .to_string();
                                        let arguments = tc
                                            .get("function")
                                            .and_then(|f| f.get("arguments"))
                                            .cloned()
                                            .unwrap_or_else(|| serde_json::json!({}));
                                        let id = uuid::Uuid::new_v4().to_string(); // Ollama sometimes doesn't provide IDs
                                        let _ = tx
                                            .send(Ok(LlmChunk::ToolCall {
                                                id,
                                                name,
                                                arguments,
                                            }))
                                            .await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Ollama parse error: {} | line: {}", e, line);
                        }
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}
