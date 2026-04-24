use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{error::AppError, models::contributions_model::ContributionsResponse, state::AppState};

#[derive(Debug, Deserialize)]
pub struct ContributionsQuery {
    pub username: Option<String>,
}

pub async fn get_contributions(
    State(state): State<AppState>,
    Query(query): Query<ContributionsQuery>,
) -> Result<Json<ContributionsResponse>, AppError> {
    let username = query.username.unwrap_or_else(|| {
        state
            .contributions_service
            .get_default_username()
            .to_string()
    });

    let resp = state
        .contributions_service
        .get_contributions(&username)
        .await?;

    Ok(Json(resp))
}
