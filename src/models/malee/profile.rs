use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    // Basic Info
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,

    // Shipping
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub zip_code: Option<String>,

    // Preferences
    pub currency: Option<String>,
    pub preferred_language: Option<String>,
    pub favorite_categories: Vec<String>,

    // Personalization & Memory
    pub memories: Vec<String>,
    pub order_history: Vec<PastOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastOrder {
    pub order_ref: String,
    pub date: DateTime<Utc>,
    pub items: Vec<String>,
    pub total_lkr: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionContext {
    pub recipient_relation: Option<String>,
    pub occasion: Option<String>,
    pub budget_min_lkr: Option<i64>,
    pub budget_max_lkr: Option<i64>,
    pub preferred_city: Option<String>,
    pub preferred_delivery_date: Option<NaiveDate>,
}
