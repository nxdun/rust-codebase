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
#[tracing::instrument(skip_all)]
pub fn reduce(
    mut state: CartState,
    action: CartAction,
    max_items: usize,
) -> Result<CartState, MaleeError> {
    tracing::debug!("Reducing cart action: {:?}", action);
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
