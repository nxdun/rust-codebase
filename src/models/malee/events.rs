use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiEvent {
    SessionCreated {
        session_id: String,
    },
    Token {
        text: String,
    },
    AssistantMessageDone {
        full_text: String,
    },
    ProductCarousel {
        title: String,
        subtitle: Option<String>,
        items: Vec<ProductCardView>,
    },
    ProductDetail {
        item: ProductDetailView,
    },
    CategoryGrid {
        categories: Vec<CategoryView>,
    },
    CartUpdated {
        cart: CartView,
    },
    CitySuggestions {
        query: String,
        cities: Vec<String>,
    },
    DeliveryQuote {
        city: String,
        date: String,
        rate_lkr: i64,
        deliverable: bool,
        perishable_warning: bool,
        next_available_date: Option<String>,
    },
    CheckoutForm {
        draft: CheckoutDraftView,
        missing_fields: Vec<String>,
    },
    CheckoutReady {
        pay_url: String,
        order_ref: String,
        expires_in_minutes: u32,
        cart_summary: Vec<CartItemView>,
    },
    CheckoutProgress {
        current_step: u32,
        total_steps: u32,
        step_name: String,
        missing_fields: Vec<String>,
    },
    TrackingResult {
        order_number: String,
        status: String,
        recipient: String,
        items: Vec<String>,
        timeline: Vec<TrackingEvent>,
    },
    LanguageChanged {
        mode: String,
    },
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCardView {
    pub id: String,
    pub name: String,
    pub price_lkr: i64,
    pub image_url: Option<String>,
    pub in_stock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDetailView {
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
pub struct CartView {
    pub items: Vec<CartItemView>,
    pub subtotal_lkr: i64,
    pub item_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItemView {
    pub product_id: String,
    pub name: String,
    pub price_lkr: i64,
    pub quantity: u32,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryView {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutDraftView {
    pub recipient_name: Option<String>,
    pub delivery_city: Option<String>,
    pub delivery_date: Option<String>,
    pub sender_name: Option<String>,
    pub gift_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingEvent {
    pub timestamp: String,
    pub description: String,
}
