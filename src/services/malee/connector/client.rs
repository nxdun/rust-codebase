use dashmap::DashMap;
use reqwest::Client;
use std::time::{Duration, Instant};

use crate::error::MaleeError;

use super::jsonrpc::{McpParams, McpRequest, McpResponse};
use super::tools::{
    TOOL_CHECK_DELIVERY, TOOL_CREATE_ORDER, TOOL_GET_PRODUCT, TOOL_LIST_CATEGORIES,
    TOOL_LIST_CITIES, TOOL_SEARCH_PRODUCTS, TOOL_TRACK_ORDER,
};
use super::types::{
    Category, CheckDeliveryArgs, CreateOrderArgs, DeliveryCheck, GetProductArgs,
    ListCategoriesArgs, ListCitiesArgs, OrderCreated, OrderTracking, ProductDetail, ProductSummary,
    SearchArgs, TrackOrderArgs,
};

pub const CONNECTOR_TIMEOUT_MS: u64 = 15000;

#[derive(Debug)]
pub struct MaleeConnector {
    client: Client,
    base_url: String,
    timeout: Duration,
    category_cache: DashMap<String, (Instant, Vec<Category>)>,
    city_cache: DashMap<String, (Instant, Vec<String>)>,
}

impl MaleeConnector {
    pub fn new(client: Client, base_url: String, timeout_ms: u64) -> Self {
        Self {
            client,
            base_url,
            timeout: Duration::from_millis(timeout_ms),
            category_cache: DashMap::new(),
            city_cache: DashMap::new(),
        }
    }

    async fn call_tool<T: serde::de::DeserializeOwned>(
        &self,
        tool_name: &str,
        args: impl serde::Serialize,
    ) -> Result<T, MaleeError> {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "tools/call".to_string(),
            params: McpParams {
                name: tool_name.to_string(),
                arguments: serde_json::to_value(args).map_err(|e| {
                    MaleeError::ConnectorError(format!("Failed to serialize args: {e}"))
                })?,
            },
        };

        let response = self
            .client
            .post(&self.base_url)
            .timeout(self.timeout)
            .json(&req)
            .send()
            .await
            .map_err(|e| MaleeError::ConnectorError(e.to_string()))?;

        if let Some(remaining) = response.headers().get("x-ratelimit-remaining-requests")
            && let Ok(rem_str) = remaining.to_str()
            && let Ok(rem_num) = rem_str.parse::<i32>()
            && rem_num < 5
        {
            tracing::warn!("MCP ratelimit running low: {}", rem_num);
        }

        let mcp_res: McpResponse = response
            .json()
            .await
            .map_err(|e| MaleeError::ConnectorError(format!("Failed to parse response: {e}")))?;

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

        serde_json::from_str(&text_content.text)
            .map_err(|e| MaleeError::ConnectorError(format!("Failed to parse content JSON: {e}")))
    }

    #[tracing::instrument(skip(self))]
    pub async fn search_products(
        &self,
        args: SearchArgs,
    ) -> Result<Vec<ProductSummary>, MaleeError> {
        self.call_tool(TOOL_SEARCH_PRODUCTS, args).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_product(&self, args: GetProductArgs) -> Result<ProductDetail, MaleeError> {
        self.call_tool(TOOL_GET_PRODUCT, args).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_categories(
        &self,
        args: ListCategoriesArgs,
    ) -> Result<Vec<Category>, MaleeError> {
        if let Some(cached) = self.category_cache.get("all")
            && cached.0.elapsed() < Duration::from_secs(180)
        {
            return Ok(cached.1.clone());
        }

        let result = self
            .call_tool::<Vec<Category>>(TOOL_LIST_CATEGORIES, args)
            .await?;
        self.category_cache
            .insert("all".to_string(), (Instant::now(), result.clone()));
        Ok(result)
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_cities(&self, args: ListCitiesArgs) -> Result<Vec<String>, MaleeError> {
        if let Some(cached) = self.city_cache.get("all")
            && cached.0.elapsed() < Duration::from_secs(600)
        {
            return Ok(cached.1.clone());
        }

        let result = self
            .call_tool::<Vec<String>>(TOOL_LIST_CITIES, args)
            .await?;
        self.city_cache
            .insert("all".to_string(), (Instant::now(), result.clone()));
        Ok(result)
    }

    #[tracing::instrument(skip(self))]
    pub async fn check_delivery(
        &self,
        args: CheckDeliveryArgs,
    ) -> Result<DeliveryCheck, MaleeError> {
        self.call_tool(TOOL_CHECK_DELIVERY, args).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_order(&self, args: CreateOrderArgs) -> Result<OrderCreated, MaleeError> {
        self.call_tool(TOOL_CREATE_ORDER, args).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn track_order(&self, args: TrackOrderArgs) -> Result<OrderTracking, MaleeError> {
        self.call_tool(TOOL_TRACK_ORDER, args).await
    }
}
