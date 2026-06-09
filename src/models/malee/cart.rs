use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: String,
    pub name: String,
    pub price_lkr: i64,
    pub quantity: u32,
    pub image_url: Option<String>,
    pub is_perishable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CartState {
    pub items: Vec<CartItem>,
}

impl CartState {
    pub fn subtotal_lkr(&self) -> i64 {
        self.items
            .iter()
            .map(|item| item.price_lkr.saturating_mul(i64::from(item.quantity)))
            .sum()
    }

    pub fn item_count(&self) -> u32 {
        self.items.iter().map(|item| item.quantity).sum()
    }
}
