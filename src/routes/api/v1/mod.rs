use axum::Router;

use crate::state::AppState;

mod ytdlp_routes;

pub fn router() -> Router<AppState> {
    Router::new().merge(ytdlp_routes::router())
}
