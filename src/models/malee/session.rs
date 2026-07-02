use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::cart::CartState;
use super::checkout::CheckoutDraft;
use super::events::ProductCardView;
use super::profile::{SessionContext, UserProfile};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LanguageMode {
    #[default]
    Auto,
    English,
    Sinhala,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
    pub language_mode: LanguageMode,
    pub conversation_history: Vec<ConversationTurn>,
    pub user_profile: UserProfile,
    pub session_context: SessionContext,
    pub cart: CartState,
    pub checkout_draft: CheckoutDraft,
    pub last_products: Vec<ProductCardView>,
    pub order_last_created_at: Option<DateTime<Utc>>,
    pub active_llm_index: usize,
}
