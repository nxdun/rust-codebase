use dashmap::DashMap;
use reqwest::Client;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::error::MaleeError;

use super::jsonrpc::{McpParams, McpRequest, McpResponse};
use super::tools::{
    TOOL_CHECK_DELIVERY, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT, TOOL_LIST_CATEGORIES,
    TOOL_LIST_CITIES, TOOL_SEARCH_PRODUCTS, TOOL_TRACK_ORDER,
};
use super::types::{
    Category, CategoryResponse, CheckDeliveryArgs, CreateOrderArgs, DeliveryCheck, GetProductArgs,
    ListCategoriesArgs, ListCitiesArgs, ListCitiesResponse, OrderCreated, OrderTracking,
    ProductDetail, ProductSummary, SearchArgs, SearchResponse, TrackOrderArgs,
};

const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_MS: u64 = 500;

#[derive(Debug)]
pub struct MaleeConnector {
    client: Client,
    base_url: String,
    timeout: Duration,
    category_cache: DashMap<String, (Instant, Vec<Category>)>,
    city_cache: DashMap<String, (Instant, Vec<String>)>,
    mcp_sessions: DashMap<String, String>,
}

impl MaleeConnector {
    pub fn new(client: Client, base_url: String, timeout_ms: u64) -> Self {
        Self {
            client,
            base_url,
            timeout: Duration::from_millis(timeout_ms),
            category_cache: DashMap::new(),
            city_cache: DashMap::new(),
            mcp_sessions: DashMap::new(),
        }
    }

    async fn initialize_mcp(&self, user_session_id: &str) -> Result<String, MaleeError> {
        tracing::info!(user_session_id, "Initializing new MCP session");

        let init_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "nadzu-backend",
                    "version": "1.0.0"
                }
            }
        });

        let response = self
            .client
            .post(&self.base_url)
            .header("Accept", "application/json, text/event-stream")
            .json(&init_req)
            .send()
            .await
            .map_err(|e| MaleeError::ConnectorError(format!("Init network error: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(MaleeError::ConnectorError(format!(
                "Init HTTP {status}: {body}"
            )));
        }

        let mcp_id = response
            .headers()
            .get("mcp-session-id")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                MaleeError::ConnectorError("No mcp-session-id in init response".to_string())
            })?
            .to_string();

        Ok(mcp_id)
    }

    #[allow(clippy::too_many_lines)]
    async fn call_tool<T: serde::de::DeserializeOwned>(
        &self,
        tool_name: &str,
        args: impl serde::Serialize,
        session_id: &str,
    ) -> Result<T, MaleeError> {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "tools/call".to_string(),
            params: McpParams {
                name: tool_name.to_string(),
                arguments: serde_json::json!({
                    "params": serde_json::to_value(&args).map_err(|e| {
                        MaleeError::ConnectorError(format!("Failed to serialize args: {e}"))
                    })?
                }),
            },
        };

        let mut attempt = 0;
        let mut delay_ms = INITIAL_RETRY_MS;

        loop {
            attempt += 1;

            // Get or initialize MCP session
            let mcp_session_id = if let Some(id) = self.mcp_sessions.get(session_id) {
                id.clone()
            } else {
                let id = self.initialize_mcp(session_id).await?;
                self.mcp_sessions.insert(session_id.to_string(), id.clone());
                id
            };

            let response_res = self
                .client
                .post(&self.base_url)
                .header("Accept", "application/json, text/event-stream")
                .header("mcp-session-id", &mcp_session_id)
                .timeout(self.timeout)
                .json(&req)
                .send()
                .await;

            let response = match response_res {
                Ok(resp) => resp,
                Err(e) => {
                    if attempt > MAX_RETRIES {
                        return Err(MaleeError::ConnectorError(format!("Network error: {e}")));
                    }
                    tracing::warn!(
                        "Connector network error (attempt {}/{}): {}. Retrying in {}ms...",
                        attempt,
                        MAX_RETRIES,
                        e,
                        delay_ms
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;
                    continue;
                }
            };

            let status = response.status();

            if let Some(remaining) = response.headers().get("x-ratelimit-remaining-requests")
                && let Ok(rem_str) = remaining.to_str()
                && let Ok(rem_num) = rem_str.parse::<i32>()
                && rem_num < 5
            {
                tracing::warn!("MCP ratelimit running low: {}", rem_num);
            }

            let body_text = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    if attempt > MAX_RETRIES {
                        return Err(MaleeError::ConnectorError(format!(
                            "Failed to read response body: {e}"
                        )));
                    }
                    tracing::warn!(
                        "Connector read error (attempt {}/{}): {}. Retrying in {}ms...",
                        attempt,
                        MAX_RETRIES,
                        e,
                        delay_ms
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;
                    continue;
                }
            };

            if !status.is_success() {
                if body_text.contains("Session not found") {
                    tracing::warn!(
                        "MCP session expired for {}, clearing and retrying...",
                        session_id
                    );
                    self.mcp_sessions.remove(session_id);
                }

                if attempt > MAX_RETRIES {
                    return Err(MaleeError::ConnectorError(format!(
                        "HTTP {status} Error. Body: {body_text}"
                    )));
                }
                tracing::warn!(
                    "Connector HTTP {status} (attempt {attempt}/{MAX_RETRIES}). Body: {body_text}. Retrying in {delay_ms}ms...",
                );
                sleep(Duration::from_millis(delay_ms)).await;
                delay_ms *= 2;
                continue;
            }

            // Strip SSE data prefix if present
            let json_text = if body_text.contains("event: message") {
                body_text
                    .lines()
                    .find(|l| l.starts_with("data: "))
                    .map(|l| l.trim_start_matches("data: "))
                    .ok_or_else(|| {
                        MaleeError::ConnectorError("Malformed SSE: missing data".to_string())
                    })?
            } else {
                &body_text
            };

            let mcp_res: McpResponse = match serde_json::from_str(json_text) {
                Ok(res) => res,
                Err(e) => {
                    if attempt > MAX_RETRIES {
                        return Err(MaleeError::ConnectorError(format!(
                            "Failed to parse response: {e}. Body: {body_text}"
                        )));
                    }
                    tracing::warn!(
                        "Connector parse error (attempt {attempt}/{MAX_RETRIES}): {e}. Body: {body_text}. Retrying in {delay_ms}ms...",
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;
                    continue;
                }
            };

            if let Some(err) = mcp_res.error {
                return Err(MaleeError::ConnectorError(err.message));
            }

            let result = mcp_res
                .result
                .ok_or_else(|| MaleeError::ConnectorError("No result in response".to_string()))?;

            if result.is_error == Some(true) {
                let msg = result
                    .content
                    .first()
                    .map(|c| c.text.clone())
                    .unwrap_or_default();
                return Err(MaleeError::ConnectorError(msg));
            }

            let text_content = result
                .content
                .first()
                .ok_or_else(|| MaleeError::ConnectorError("No content in result".to_string()))?;

            match serde_json::from_str::<T>(&text_content.text) {
                Ok(val) => return Ok(val),
                Err(e) => {
                    // If it's a known non-JSON error message from Kapruka, return it as a string
                    // if T allows it, or just wrap it in a MaleeError that the agent can see.
                    if text_content.text.contains("No products found")
                        || text_content.text.contains("Error")
                    {
                        return Err(MaleeError::ConnectorError(text_content.text.clone()));
                    }

                    return Err(MaleeError::ConnectorError(format!(
                        "Failed to parse content JSON: {e}. Text: {}",
                        text_content.text
                    )));
                }
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn search_products(
        &self,
        args: SearchArgs,
        session_id: &str,
    ) -> Result<Vec<ProductSummary>, MaleeError> {
        let res: SearchResponse = self
            .call_tool(TOOL_SEARCH_PRODUCTS, args, session_id)
            .await?;
        Ok(res.results)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_product(
        &self,
        args: GetProductArgs,
        session_id: &str,
    ) -> Result<ProductDetail, MaleeError> {
        self.call_tool(TOOL_GET_PRODUCT, args, session_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_categories(
        &self,
        args: ListCategoriesArgs,
        session_id: &str,
    ) -> Result<Vec<Category>, MaleeError> {
        if let Some(cached) = self.category_cache.get("all")
            && cached.0.elapsed() < Duration::from_mins(3)
        {
            return Ok(cached.1.clone());
        }

        let res: CategoryResponse = self
            .call_tool(TOOL_LIST_CATEGORIES, args, session_id)
            .await?;
        self.category_cache
            .insert("all".to_string(), (Instant::now(), res.categories.clone()));
        Ok(res.categories)
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_cities(
        &self,
        args: ListCitiesArgs,
        session_id: &str,
    ) -> Result<Vec<String>, MaleeError> {
        if let Some(cached) = self.city_cache.get("all")
            && cached.0.elapsed() < Duration::from_mins(10)
        {
            return Ok(cached.1.clone());
        }

        let res: ListCitiesResponse = self.call_tool(TOOL_LIST_CITIES, args, session_id).await?;
        let names: Vec<String> = res.cities.into_iter().map(|c| c.name).collect();
        self.city_cache
            .insert("all".to_string(), (Instant::now(), names.clone()));
        Ok(names)
    }

    #[tracing::instrument(skip(self))]
    pub async fn check_delivery(
        &self,
        args: CheckDeliveryArgs,
        session_id: &str,
    ) -> Result<DeliveryCheck, MaleeError> {
        self.call_tool(TOOL_CHECK_DELIVERY, args, session_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_order(
        &self,
        args: CreateOrderArgs,
        session_id: &str,
    ) -> Result<OrderCreated, MaleeError> {
        self.call_tool(TOOL_CREATE_ORDER, args, session_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn track_order(
        &self,
        args: TrackOrderArgs,
        session_id: &str,
    ) -> Result<OrderTracking, MaleeError> {
        self.call_tool(TOOL_TRACK_ORDER, args, session_id).await
    }
}
