use axum::{
    middleware,
    Router,
    routing::{get, post},
};

use crate::{
    controllers::api::v1::ytdlp_controller,
    middleware::captcha::verify_captcha_token,
    state::AppState,
};

pub fn router(state: AppState) -> Router<AppState> {
    let public_routes = Router::new()
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
        );

    let submit_route = Router::new()
        .route("/api/v1/ytdlp", post(ytdlp_controller::create_download_job))
        .layer(middleware::from_fn_with_state(state, verify_captcha_token));

    Router::new().merge(public_routes).merge(submit_route)
}
