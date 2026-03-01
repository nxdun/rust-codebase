use axum::Router;

use crate::state::AppState;

mod ytdlp_routes;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().merge(ytdlp_routes::router(state))
}
