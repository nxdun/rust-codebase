use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{error::AppError, models::contributions::ContributionsResponse, state::AppState};

/// Request struct
#[derive(Debug, Deserialize)]
pub struct ContributionsQuery {
    pub username: Option<String>,
}

/// Retrieves GitHub contributions for a specific user.
/// Defaults to the configured system user if none is provided.
pub async fn get_contributions(
    State(state): State<AppState>,
    Query(query): Query<ContributionsQuery>,
) -> Result<Json<ContributionsResponse>, AppError> {
    let username = query
        .username
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map_or_else(
            || {
                state
                    .contributions_service
                    .get_default_username()
                    .to_string()
            },
            ToOwned::to_owned,
        );

    let resp = state
        .contributions_service
        .get_contributions(&username)
        .await?;

    Ok(Json(resp))
}
