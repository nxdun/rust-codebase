use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppError, models::malee::profile::UserProfile, state::AppState};

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub profile: UserProfile,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub session_id: Uuid,
    pub profile: UserProfile,
}

#[tracing::instrument(skip(state, body))]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, AppError> {
    let mut session = state
        .malee_service
        .session_store
        .get(&id)
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    session.user_profile = body.profile;
    session.updated_at = chrono::Utc::now();

    state.malee_service.session_store.upsert(session.clone());

    Ok(Json(ProfileResponse {
        session_id: id,
        profile: session.user_profile,
    }))
}

#[tracing::instrument(skip(state))]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProfileResponse>, AppError> {
    let session = state
        .malee_service
        .session_store
        .get(&id)
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    Ok(Json(ProfileResponse {
        session_id: id,
        profile: session.user_profile,
    }))
}
