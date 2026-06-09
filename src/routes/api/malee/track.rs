#![allow(unreachable_pub, clippy::redundant_pub_crate)]
use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;

use crate::{error::AppError, services::malee::connector::types::TrackOrderArgs, state::AppState};

#[derive(Debug, Deserialize)]
pub struct TrackRequest {
    pub order_id: String,
}

pub async fn handler(
    State(state): State<AppState>,
    Json(body): Json<TrackRequest>,
) -> Result<impl IntoResponse, AppError> {
    let args = TrackOrderArgs {
        order_id: body.order_id,
    };
    let res = state.malee_service.connector.track_order(args).await?;

    Ok(Json(res))
}
