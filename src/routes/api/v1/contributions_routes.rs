use axum::{Router, routing::get};

use crate::{controllers::api::v1::contributions_controller::get_contributions, state::AppState};

pub fn router(_state: AppState) -> Router<AppState> {
    Router::new().route("/", get(get_contributions))
}
