use axum::{
    Router,
    routing::{get, post},
};

use crate::{controllers::api::v1::ytdlp_controller, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/ytdlp", post(ytdlp_controller::create_download_job))
        .route(
            "/api/v1/ytdlp/jobs",
            get(ytdlp_controller::list_download_jobs),
        )
        .route(
            "/api/v1/ytdlp/jobs/{id}",
            get(ytdlp_controller::get_download_job),
        )
        .route(
            "/api/v1/ytdlp/download/{id}",
            get(ytdlp_controller::download_file),
        )
}
