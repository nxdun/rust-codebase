#![allow(unreachable_pub, clippy::redundant_pub_crate)]
use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::malee::{cart::CartItem, checkout::DeliveryInfo, events::CartView},
    services::malee::cart::reducer::{CartAction, reduce},
    state::AppState,
};
pub const MAX_CART_ITEMS: usize = 20;
pub const GIFT_NOTE_MAX_CHARS: usize = 240;

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    pub session_id: Uuid,
    pub action: String,
    pub payload: Value,
}

#[tracing::instrument(skip(state, body))]
#[allow(clippy::too_many_lines)]
pub async fn handler(
    State(state): State<AppState>,
    Json(body): Json<ActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mut session = state
        .malee_service
        .session_store
        .get(&body.session_id)
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    let max_items = MAX_CART_ITEMS;

    match body.action.as_str() {
        "add_to_cart" => {
            let product: CartItem = serde_json::from_value(body.payload)
                .map_err(|e| AppError::Validation(format!("Invalid payload: {e}")))?;
            session.cart = reduce(session.cart, CartAction::AddItem { product }, max_items)?;
        }
        "remove_from_cart" => {
            if let Some(id) = body.payload.get("product_id").and_then(|v| v.as_str()) {
                session.cart = reduce(
                    session.cart,
                    CartAction::RemoveItem {
                        product_id: id.to_string(),
                    },
                    max_items,
                )?;
            }
        }
        "set_quantity" => {
            if let (Some(id), Some(qty)) = (
                body.payload.get("product_id").and_then(|v| v.as_str()),
                body.payload.get("quantity").and_then(Value::as_u64),
            ) {
                session.cart = reduce(
                    session.cart,
                    CartAction::SetQuantity {
                        product_id: id.to_string(),
                        quantity: u32::try_from(qty).unwrap_or(u32::MAX),
                    },
                    max_items,
                )?;
            }
        }
        "set_delivery_city" => {
            if let Some(city) = body.payload.get("city").and_then(|v| v.as_str()) {
                if session.checkout_draft.delivery.is_none() {
                    session.checkout_draft.delivery = Some(DeliveryInfo::default());
                }
                if let Some(d) = &mut session.checkout_draft.delivery {
                    d.city = city.to_string();
                }
            }
        }
        "set_delivery_date" => {
            if let Some(date_str) = body.payload.get("date").and_then(|v| v.as_str())
                && let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            {
                if session.checkout_draft.delivery.is_none() {
                    session.checkout_draft.delivery = Some(DeliveryInfo::default());
                }
                if let Some(d) = &mut session.checkout_draft.delivery {
                    d.date = date;
                }
            }
        }
        "set_gift_note" => {
            if let Some(note) = body.payload.get("note").and_then(|v| v.as_str()) {
                let max_chars = GIFT_NOTE_MAX_CHARS;
                let note = if note.len() > max_chars {
                    note.chars().take(max_chars).collect()
                } else {
                    note.to_string()
                };
                session.checkout_draft.gift_message = Some(note);
            }
        }
        "set_language" => {
            if let Some(mode) = body.payload.get("mode").and_then(|v| v.as_str()) {
                session.language_mode = match mode {
                    "english" => crate::models::malee::session::LanguageMode::English,
                    "sinhala" => crate::models::malee::session::LanguageMode::Sinhala,
                    "mixed" => crate::models::malee::session::LanguageMode::Mixed,
                    _ => crate::models::malee::session::LanguageMode::Auto,
                };
            }
        }
        "clear_cart" => {
            session.cart = reduce(session.cart, CartAction::Clear, max_items)?;
        }
        _ => {
            return Err(AppError::Validation(format!(
                "Unknown action: {}",
                body.action
            )));
        }
    }

    let cart_view = CartView::from(&session.cart);

    state.malee_service.session_store.upsert(session);

    Ok(Json(cart_view))
}
