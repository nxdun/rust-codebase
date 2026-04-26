use axum::Router;

use crate::state::AppState;

mod contributions_routes;
mod ytdlp_routes;

#[allow(unreachable_pub)]
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(ytdlp_routes::router(state.clone()))
        .nest("/api/v1/contributions", contributions_routes::router(state))
}
