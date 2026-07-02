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
    models::malee::events::{CartView, CheckoutDraftView},
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

    let cart_view = CartView::from(&session.cart);

    let checkout_draft = CheckoutDraftView::from(&session.checkout_draft);

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
