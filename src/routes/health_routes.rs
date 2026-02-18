use crate::{controllers::health_controller, state::AppState};
use axum::{Router, routing::get};

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health_controller::check_health))
}
