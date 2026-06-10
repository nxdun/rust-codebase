#![allow(unreachable_pub, clippy::redundant_pub_crate)]

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::malee::events::{CartItemView, CartView, CheckoutDraftView},
    state::AppState,
};

#[derive(Serialize)]
pub struct SessionView {
    pub session_id: String,
    pub language_mode: String,
    pub cart: CartView,
    pub checkout_draft: CheckoutDraftView,
    pub last_product_ids: Vec<String>,
    pub active_llm_index: usize,
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let session = state
        .malee_service
        .session_store
        .get(&id)
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    let cart_view = CartView {
        items: session
            .cart
            .items
            .iter()
            .map(|i| CartItemView {
                product_id: i.product_id.clone(),
                name: i.name.clone(),
                price_lkr: i.price_lkr,
                quantity: i.quantity,
                image_url: i.image_url.clone(),
            })
            .collect(),
        subtotal_lkr: session.cart.subtotal_lkr(),
        item_count: session.cart.item_count(),
    };

    let checkout_draft = CheckoutDraftView {
        recipient_name: session.checkout_draft.recipient.map(|r| r.name),
        delivery_city: session
            .checkout_draft
            .delivery
            .as_ref()
            .map(|d| d.city.clone()),
        delivery_date: session.checkout_draft.delivery.map(|d| d.date.to_string()),
        sender_name: session.checkout_draft.sender.map(|s| s.name),
        gift_message: session.checkout_draft.gift_message,
    };

    let view = SessionView {
        session_id: session.session_id.to_string(),
        language_mode: format!("{:?}", session.language_mode).to_lowercase(),
        cart: cart_view,
        checkout_draft,
        last_product_ids: session.last_products.into_iter().map(|p| p.id).collect(),
        active_llm_index: session.active_llm_index,
    };

    Ok(Json(view))
}

pub async fn reset(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    state.malee_service.session_store.delete(&id);
    axum::http::StatusCode::NO_CONTENT
}
