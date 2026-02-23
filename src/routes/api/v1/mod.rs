use axum::Router;

use crate::state::AppState;

mod captcha_routes;
mod ytdlp_routes;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(captcha_routes::router())
        .merge(ytdlp_routes::router())
}
