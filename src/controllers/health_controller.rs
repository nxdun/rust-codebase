use crate::{error::AppError, models::health_model::Health, state::AppState};
use axum::{Json, extract::State, http::StatusCode};
use std::path::Path;

pub async fn check_health(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Health>), AppError> {
    let cookies = state
        .config
        .ytdlp_cookies_file
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .is_some_and(|path| Path::new(path).exists());

    let data = Health::ok(cookies);
    Ok((StatusCode::OK, Json(data)))
}
