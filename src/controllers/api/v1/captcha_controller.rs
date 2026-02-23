use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;

use crate::{models::captcha_model::*, state::AppState};

pub async fn verify_captcha(
    State(state): State<AppState>,
    Json(payload): Json<CaptchaVerifyRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let captcha = match payload.captcha.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) {
        Some(token) => token,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "message": "CapToken is required" })),
            );
        }
    };

    let secret_key = match state.config.captcha_secret_key.as_deref() {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "CAPTCHA_SECRET_KEY is not configured" })),
            );
        }
    };

    let google_url = "https://www.google.com/recaptcha/api/siteverify";
    let client = &state.http_client;

    let response = match client
        .post(google_url)
        .timeout(std::time::Duration::from_secs(10))
        .form(&[("secret", secret_key), ("response", captcha.as_str())])
        .send()
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to send captcha verification request: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "message": "Failed to verify CapToken" })),
            );
        }
    };

    let body = match response.json::<GoogleCaptchaVerifyResponse>().await {
        Ok(parsed) => parsed,
        Err(e) => {
            tracing::error!("Failed to parse captcha provider response: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "message": "Failed to parse captcha provider response" })),
            );
        }
    };

    if body.success {
        (
            StatusCode::OK,
            Json(json!({ "message": "CapToken is valid", "success": true })),
        )
    } else {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "message": "CapToken is invalid", "success": false })),
        )
    }
}
