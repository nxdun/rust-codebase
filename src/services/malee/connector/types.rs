use serde::{Deserialize, Serialize};

fn default_response_format() -> String {
    "json".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    #[serde(rename = "q", skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(rename = "category", skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default = "default_response_format")]
    pub response_format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<ProductSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "price")]
    pub price_info: PriceInfo,
    pub image_url: Option<String>,
    pub in_stock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInfo {
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProductArgs {
    pub product_id: String,
    #[serde(default = "default_response_format")]
    pub response_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "price")]
    pub price_info: PriceInfo,
    pub images: Vec<String>,
    pub in_stock: bool,
    #[serde(default)]
    pub is_perishable: bool,
    pub attributes: Option<ProductAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductAttributes {
    pub vendor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCategoriesArgs {
    #[serde(default = "default_response_format")]
    pub response_format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryResponse {
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCitiesArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default = "default_response_format")]
    pub response_format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListCitiesResponse {
    pub cities: Vec<CityInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityInfo {
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDeliveryArgs {
    pub city: String,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    #[serde(default = "default_response_format")]
    pub response_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryCheck {
    pub available: bool,
    pub rate: f64,
    pub perishable_warning: Option<String>,
    pub next_available_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderArgs {
    pub cart: Vec<McpCartItem>,
    pub recipient: McpRecipient,
    pub delivery: McpDelivery,
    pub sender: McpSender,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gift_message: Option<String>,
    #[serde(default = "default_lkr")]
    pub currency: String,
    #[serde(default = "default_response_format")]
    pub response_format: String,
}

fn default_lkr() -> String {
    "LKR".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCartItem {
    pub product_id: String,
    pub quantity: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icing_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRecipient {
    pub name: String,
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDelivery {
    pub address: String,
    pub city: String,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSender {
    pub name: String,
    #[serde(skip_serializing)]
    pub email: Option<String>,
    #[serde(default)]
    pub anonymous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSummary {
    pub items_total: f64,
    pub delivery_fee: f64,
    pub addons_total: f64,
    pub grand_total: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreated {
    pub order_ref: String,
    #[serde(rename = "checkout_url")]
    pub pay_url: String,
    pub summary: OrderSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackOrderArgs {
    pub order_number: String,
    #[serde(default = "default_response_format")]
    pub response_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderTracking {
    pub status: String,
    pub recipient: RecipientSummary,
    pub items: Vec<OrderItemSummary>,
    pub progress: Vec<crate::models::malee::events::TrackingEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipientSummary {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemSummary {
    pub name: String,
}

pub type Product = ProductSummary;
pub type DeliveryQuote = DeliveryCheck;
pub type OrderResult = OrderCreated;
pub type TrackingDetails = OrderTracking;
