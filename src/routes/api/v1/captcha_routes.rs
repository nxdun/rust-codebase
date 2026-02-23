use axum::{Router, routing::post};

use crate::{controllers::api::v1::captcha_controller, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/captcha/verify",
        post(captcha_controller::verify_captcha),
    )
}
