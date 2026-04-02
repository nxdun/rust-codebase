use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::{
    controllers::api::v1::ytdlp_controller,
    middleware::{auth::require_api_key, captcha::verify_captcha_token},
    state::AppState,
};

pub const ROUTE_YTDLP_BASE: &str = "/api/v1/ytdlp";

pub fn router(state: AppState) -> Router<AppState> {
    let nested_routes = Router::new()
        .route("/sites", get(ytdlp_controller::get_supported_sites))
        .route("/jobs/{id}", get(ytdlp_controller::get_download_job))
        .route(
            "/jobs/{id}/stream",
            get(ytdlp_controller::stream_download_progress),
        )
        .route("/download/{id}", get(ytdlp_controller::download_file))
        .merge(
            Router::new()
                .route("/jobs", get(ytdlp_controller::list_download_jobs))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_api_key,
                )),
        );

    let submit_route = Router::new()
        .route(
            ROUTE_YTDLP_BASE,
            post(ytdlp_controller::create_download_job),
        )
        .layer(middleware::from_fn_with_state(state, verify_captcha_token));

    Router::new()
        .nest(ROUTE_YTDLP_BASE, nested_routes)
        .merge(submit_route)
}
