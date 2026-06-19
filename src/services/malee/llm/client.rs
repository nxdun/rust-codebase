use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::MaleeError;

/// Maximum seconds to wait for the next SSE chunk before treating the stream as stalled.
const STREAM_CHUNK_TIMEOUT_SECS: u64 = 45;

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

/// Accumulates streamed tool call deltas into complete tool calls.
///
/// OpenAI-compatible APIs stream tool calls as incremental argument
/// fragments across multiple SSE chunks. This struct buffers the
/// fragments and emits a complete `LlmChunk::ToolCall` when a new
/// tool call starts or the stream ends.
struct ToolCallAccumulator {
    id: Option<String>,
    name: Option<String>,
    args: String,
}

impl ToolCallAccumulator {
    const fn new() -> Self {
        Self {
            id: None,
            name: None,
            args: String::new(),
        }
    }

    /// Process a `tool_calls` delta from the LLM stream.
    ///
    /// Returns a completed `LlmChunk::ToolCall` if a new tool call starts,
    /// flushing the previously accumulated one.
    fn process_delta(&mut self, tc: &serde_json::Value) -> Option<LlmChunk> {
        let mut flushed = None;

        // A new tool call begins when the delta carries an `id` field.
        if let Some(new_id) = tc.get("id").and_then(serde_json::Value::as_str) {
            // Flush the previous tool call if one was being accumulated.
            if let Some(prev_id) = self.id.take()
                && let (Some(name), Ok(arguments)) =
                    (self.name.take(), serde_json::from_str(&self.args))
            {
                flushed = Some(LlmChunk::ToolCall {
                    id: prev_id,
                    name,
                    arguments,
                });
            }

            self.id = Some(new_id.to_string());
            self.name = tc
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string);
            self.args.clear();
        }

        // Append any argument fragment.
        if let Some(fn_obj) = tc.get("function")
            && let Some(fragment) = fn_obj.get("arguments").and_then(serde_json::Value::as_str)
        {
            self.args.push_str(fragment);
        }

        flushed
    }

    /// Flush any pending tool call (called at stream end or `[DONE]`).
    fn flush(&mut self) -> Option<LlmChunk> {
        let prev_id = self.id.take()?;
        let name = self.name.take()?;
        let arguments: serde_json::Value = serde_json::from_str(&self.args).ok()?;
        self.args.clear();
        Some(LlmChunk::ToolCall {
            id: prev_id,
            name,
            arguments,
        })
    }
}

/// Generic client for any provider that speaks the `OpenAI` chat-completions
/// streaming protocol (Groq, `OpenAI`, Cerebras, Fireworks, Google, Nvidia).
#[derive(Debug)]
pub struct OpenAiCompatibleClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAiCompatibleClient {
    /// Creates a new client targeting an OpenAI-compatible endpoint.
    pub const fn new(client: Client, api_key: String, base_url: String, model: String) -> Self {
        Self {
            client,
            api_key,
            base_url,
            model,
        }
    }

    #[tracing::instrument(skip(self, messages, tools), fields(model = %self.model, messages_len = messages.len(), tools_len = tools.len()))]
    #[allow(clippy::too_many_lines, clippy::type_complexity)]
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
            "Sending request to LLM: {} | Body: {}",
            self.model,
            serde_json::to_string(&body).unwrap_or_default()
        );
        let response = self
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
                return Err(MaleeError::LlmRateLimited {
                    provider: self.model.clone(),
                    retry_after_ms: None,
                });
            }
            return Err(MaleeError::LlmError(format!("HTTP {status} - {text}")));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut response = response;
            let mut accumulator = ToolCallAccumulator::new();
            let mut buffer = String::new();

            tracing::debug!("Starting LLM stream processing");
            loop {
                match tokio::time::timeout(
                    Duration::from_secs(STREAM_CHUNK_TIMEOUT_SECS),
                    response.chunk(),
                )
                .await
                {
                    Ok(Ok(Some(chunk))) => {
                        let text = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&text);

                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer.drain(..=line_end);

                            if line.is_empty() {
                                continue;
                            }
                            if let Some(data) =
                                crate::services::malee::sse::parser::parse_sse_line(&line)
                            {
                                if data == "[DONE]" {
                                    tracing::debug!("LLM stream reached [DONE]");
                                    if let Some(tool_call) = accumulator.flush() {
                                        let _ = tx.send(Ok(tool_call)).await;
                                    }
                                    let _ = tx.send(Ok(LlmChunk::Done)).await;
                                    return;
                                }

                                match serde_json::from_str::<serde_json::Value>(data) {
                                    Ok(json) => {
                                        if let Some(err) = json.get("error") {
                                            tracing::error!("LLM stream error field: {}", err);
                                            let _ = tx
                                                .send(Err(MaleeError::LlmError(err.to_string())))
                                                .await;
                                            return;
                                        }

                                        if let Some(choices) = json
                                            .get("choices")
                                            .and_then(serde_json::Value::as_array)
                                        {
                                            for choice in choices {
                                                if let Some(delta) = choice.get("delta") {
                                                    if let Some(content) = delta
                                                        .get("content")
                                                        .and_then(serde_json::Value::as_str)
                                                        && !content.is_empty()
                                                        && tx
                                                            .send(Ok(LlmChunk::Token(
                                                                content.to_string(),
                                                            )))
                                                            .await
                                                            .is_ok()
                                                    {
                                                    }

                                                    if let Some(tool_calls) = delta
                                                        .get("tool_calls")
                                                        .and_then(serde_json::Value::as_array)
                                                    {
                                                        for tc in tool_calls {
                                                            if let Some(flushed) =
                                                                accumulator.process_delta(tc)
                                                                && tx
                                                                    .send(Ok(flushed))
                                                                    .await
                                                                    .is_err()
                                                            {
                                                                return;
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
                                        let _ = tx
                                            .send(Err(MaleeError::LlmError(format!(
                                                "Stream JSON parse error: {e}"
                                            ))))
                                            .await;
                                        return;
                                    }
                                }
                            } else {
                                tracing::debug!("Ignoring non-data SSE line: {}", line);
                            }
                        }
                    }
                    Ok(Ok(None)) => break,
                    Ok(Err(e)) => {
                        tracing::error!("LLM stream read error: {}", e);
                        let _ = tx.send(Err(MaleeError::LlmError(e.to_string()))).await;
                        break;
                    }
                    Err(_) => {
                        let _ = tx
                            .send(Err(MaleeError::LlmStreamTimeout {
                                seconds: STREAM_CHUNK_TIMEOUT_SECS,
                            }))
                            .await;
                        break;
                    }
                }
            }

            tracing::debug!("LLM chunk stream ended (Ok(None))");

            if let Some(tool_call) = accumulator.flush() {
                let _ = tx.send(Ok(tool_call)).await;
            }

            let _ = tx.send(Ok(LlmChunk::Done)).await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
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
    /// Creates a new Ollama-native streaming client.
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
            loop {
                match tokio::time::timeout(
                    Duration::from_secs(STREAM_CHUNK_TIMEOUT_SECS),
                    response.chunk(),
                )
                .await
                {
                    Ok(Ok(Some(chunk))) => {
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
                                        if let Some(content) = message
                                            .get("content")
                                            .and_then(serde_json::Value::as_str)
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
                                                let id = uuid::Uuid::new_v4().to_string();
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
                                    let _ = tx
                                        .send(Err(MaleeError::LlmError(format!(
                                            "Stream JSON parse error: {e}"
                                        ))))
                                        .await;
                                    return;
                                }
                            }
                        }
                    }
                    Ok(Ok(None)) => break,
                    Ok(Err(e)) => {
                        tracing::error!("LLM stream read error: {}", e);
                        let _ = tx.send(Err(MaleeError::LlmError(e.to_string()))).await;
                        break;
                    }
                    Err(_) => {
                        let _ = tx
                            .send(Err(MaleeError::LlmStreamTimeout {
                                seconds: STREAM_CHUNK_TIMEOUT_SECS,
                            }))
                            .await;
                        break;
                    }
                }
            }

            // Ensure Done is always sent even if stream ends without "done": true
            let _ = tx.send(Ok(LlmChunk::Done)).await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}
