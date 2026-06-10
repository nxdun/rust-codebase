use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckoutDraft {
    pub recipient: Option<RecipientInfo>,
    pub delivery: Option<DeliveryInfo>,
    pub sender: Option<SenderInfo>,
    pub gift_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipientInfo {
    pub name: String,
    pub phone: String,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryInfo {
    #[serde(rename = "delivery_date")]
    pub date: NaiveDate,
    pub city: String,
    pub quote_status: QuoteStatus,
}

impl Default for DeliveryInfo {
    #[allow(clippy::unwrap_used)]
    fn default() -> Self {
        Self {
            date: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            city: String::new(),
            quote_status: QuoteStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderInfo {
    pub name: String,
    pub email: String,
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum QuoteStatus {
    #[default]
    Pending,
    Quoted {
        rate_lkr: i64,
    },
    NotDeliverable,
}
