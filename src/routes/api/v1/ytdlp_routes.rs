use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::{
    controllers::api::v1::ytdlp_controller,
    middleware::{auth::require_api_key, captcha::verify_captcha_token},
    state::AppState,
};

pub(super) fn router(state: AppState) -> Router<AppState> {
    let public_routes = Router::new()
        .route(
            "/api/v1/ytdlp/sites",
            get(ytdlp_controller::get_supported_sites),
        )
        .route(
            "/api/v1/ytdlp/jobs/{id}",
            get(ytdlp_controller::get_download_job),
        )
        .route(
            "/api/v1/ytdlp/jobs/{id}/stream",
            get(ytdlp_controller::stream_download_progress),
        )
        .route(
            "/api/v1/ytdlp/download/{id}",
            get(ytdlp_controller::download_file),
        );

    let submit_route = Router::new()
        .route("/api/v1/ytdlp", post(ytdlp_controller::create_download_job))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            verify_captcha_token,
        ));

    let auth_routes = Router::new()
        .route(
            "/api/v1/ytdlp/jobs",
            get(ytdlp_controller::list_download_jobs),
        )
        .layer(middleware::from_fn_with_state(state, require_api_key));

    Router::new()
        .merge(public_routes)
        .merge(submit_route)
        .merge(auth_routes)
}
