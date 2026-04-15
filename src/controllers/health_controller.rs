use crate::{error::AppError, models::health_model::Health, state::AppState};
use axum::{Json, extract::State};

/// Health check endpoint.
pub async fn check_health(State(_state): State<AppState>) -> Result<Json<Health>, AppError> {
    Ok(Json(Health::ok()))
}
