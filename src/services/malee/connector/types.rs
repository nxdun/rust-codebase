use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: Option<String>,
    pub category_id: Option<String>,
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSummary {
    pub id: String,
    pub name: String,
    pub price_lkr: i64,
    pub image_url: Option<String>,
    pub in_stock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProductArgs {
    pub product_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_lkr: i64,
    pub image_urls: Vec<String>,
    pub in_stock: bool,
    pub is_perishable: bool,
    pub vendor_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCategoriesArgs {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCitiesArgs {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDeliveryArgs {
    pub city: String,
    pub date: String,
    pub is_perishable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryCheck {
    pub deliverable: bool,
    pub rate_lkr: i64,
    pub perishable_warning: bool,
    pub next_available_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderArgs {
    pub items: Vec<crate::models::malee::cart::CartItem>,
    pub recipient: crate::models::malee::checkout::RecipientInfo,
    pub delivery: crate::models::malee::checkout::DeliveryInfo,
    pub sender: crate::models::malee::checkout::SenderInfo,
    pub gift_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreated {
    pub order_ref: String,
    pub pay_url: String,
    pub expires_in_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackOrderArgs {
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderTracking {
    pub status: String,
    pub recipient: String,
    pub items: Vec<String>,
    pub timeline: Vec<crate::models::malee::events::TrackingEvent>,
}
