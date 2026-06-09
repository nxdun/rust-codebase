use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: McpParams,
}

#[derive(Debug, Serialize)]
pub struct McpParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<McpResult>,
    pub error: Option<McpError>,
}

#[derive(Debug, Deserialize)]
pub struct McpResult {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}
