use crate::{
    controllers::{error_controller::not_found_handler, root_controller::root_handler},
    state::AppState,
};
use axum::{Router, routing::get};

mod api;
mod health_routes;
mod validation_routes;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(root_handler))
        .merge(health_routes::router())
        .merge(validation_routes::router())
        .merge(api::v1::router(state))
        .fallback(not_found_handler)
}
