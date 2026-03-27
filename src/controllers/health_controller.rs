use crate::{error::AppError, models::health_model::Health, state::AppState};
use axum::{Json, extract::State, http::StatusCode};
pub async fn check_health(
    State(_state): State<AppState>,
) -> Result<(StatusCode, Json<Health>), AppError> {
    let data = Health::ok();
    Ok((StatusCode::OK, Json(data)))
}