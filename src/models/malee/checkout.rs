use super::events::CheckoutDraftView;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckoutDraft {
    pub recipient: Option<RecipientInfo>,
    pub delivery: Option<DeliveryInfo>,
    pub sender: Option<SenderInfo>,
    pub gift_message: Option<String>,
    pub special_instructions: Option<String>,
    pub location_type: Option<String>, // house, apartment, office, other
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

impl From<&CheckoutDraft> for CheckoutDraftView {
    fn from(draft: &CheckoutDraft) -> Self {
        let recipient_address = draft.recipient.as_ref().map(|r| {
            format!(
                "{}{}",
                r.address_line1,
                r.address_line2
                    .as_ref()
                    .map(|l| format!(", {l}"))
                    .unwrap_or_default()
            )
        });

        Self {
            recipient_name: draft.recipient.as_ref().map(|r| r.name.clone()),
            recipient_address,
            delivery_city: draft.delivery.as_ref().map(|d| d.city.clone()),
            delivery_date: draft.delivery.as_ref().map(|d| d.date.to_string()),
            sender_name: draft.sender.as_ref().map(|s| s.name.clone()),
            gift_message: draft.gift_message.clone(),
        }
    }
}
