use crate::{error::AppError, models::health::Health, state::AppState};
use axum::{Json, extract::State};

/// Health check endpoint.
/// /health
pub async fn check_health(State(_state): State<AppState>) -> Result<Json<Health>, AppError> {
    Ok(Json(Health::ok()))
}
