use serde::{Deserialize, Serialize};

use super::events::{CartItemView, CartView};

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

impl From<&CartItem> for CartItemView {
    fn from(item: &CartItem) -> Self {
        Self {
            product_id: item.product_id.clone(),
            name: item.name.clone(),
            price_lkr: item.price_lkr,
            quantity: item.quantity,
            image_url: item.image_url.clone(),
        }
    }
}

impl From<&CartState> for CartView {
    fn from(state: &CartState) -> Self {
        Self {
            items: state.items.iter().map(CartItemView::from).collect(),
            subtotal_lkr: state.subtotal_lkr(),
            item_count: state.item_count(),
        }
    }
}
