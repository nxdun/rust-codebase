use crate::error::MaleeError;
use crate::models::malee::cart::{CartItem, CartState};

#[derive(Debug)]
pub enum CartAction {
    AddItem { product: CartItem },
    RemoveItem { product_id: String },
    SetQuantity { product_id: String, quantity: u32 },
    Clear,
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
pub fn reduce(
    mut state: CartState,
    action: CartAction,
    max_items: usize,
) -> Result<CartState, MaleeError> {
    let current_count = state.item_count();
    match action {
        CartAction::AddItem { product } => {
            if let Some(existing) = state
                .items
                .iter_mut()
                .find(|i| i.product_id == product.product_id)
            {
                if current_count + product.quantity > max_items as u32 {
                    return Err(MaleeError::CartFull(max_items));
                }
                existing.quantity += product.quantity;
            } else {
                if state.items.len() >= max_items
                    || current_count + product.quantity > max_items as u32
                {
                    return Err(MaleeError::CartFull(max_items));
                }
                state.items.push(product);
            }
        }
        CartAction::RemoveItem { product_id } => {
            state.items.retain(|i| i.product_id != product_id);
        }
        CartAction::SetQuantity {
            product_id,
            quantity,
        } => {
            if quantity == 0 {
                state.items.retain(|i| i.product_id != product_id);
            } else if let Some(existing) =
                state.items.iter_mut().find(|i| i.product_id == product_id)
            {
                let diff = quantity as i32 - existing.quantity as i32;
                if diff > 0 && current_count + diff as u32 > max_items as u32 {
                    return Err(MaleeError::CartFull(max_items));
                }
                existing.quantity = quantity;
            }
        }
        CartAction::Clear => {
            state.items.clear();
        }
    }
    Ok(state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_add_item() {
        let state = CartState::default();
        let action = CartAction::AddItem {
            product: CartItem {
                product_id: "p1".to_string(),
                name: "Test".to_string(),
                price_lkr: 1000,
                quantity: 1,
                image_url: None,
                is_perishable: false,
            },
        };
        let new_state = reduce(state, action, 10).unwrap();
        assert_eq!(new_state.items.len(), 1);
        assert_eq!(new_state.items[0].quantity, 1);
    }
}
