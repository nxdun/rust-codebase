use axum::Router;

use crate::state::AppState;

mod ytdlp_routes;

#[allow(unreachable_pub)]
pub fn router(state: AppState) -> Router<AppState> {
    Router::new().merge(ytdlp_routes::router(state))
}
