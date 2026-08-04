use crate::{middleware::api_key::require_api_key, state::AppState};
use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};

pub mod action;
pub mod chat;
pub mod profile;
pub mod session;
pub mod track;

pub fn malee_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/malee/chat", post(chat::handler))
        .route("/malee/action", post(action::handler))
        .route(
            "/malee/session/{id}",
            get(session::get).delete(session::reset),
        )
        .route("/malee/track", post(track::handler))
        .route(
            "/malee/session/{id}/profile",
            get(profile::get).put(profile::update),
        )
        .layer(from_fn_with_state(state, require_api_key))
}
