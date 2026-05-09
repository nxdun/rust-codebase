use crate::state::AppState;
use axum::{Json, extract::State};
use serde_json::{Value, json};

/// root endpoint response.
pub async fn root_handler(State(state): State<AppState>) -> Json<Value> {
    let cfg = &state.config;
    Json(json!({
        "status": "alive",
        "name": cfg.name,
        "message": format!("{} - alive and listening", cfg.name)
    }))
}
