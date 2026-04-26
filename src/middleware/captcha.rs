use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
};
use serde::Deserialize;

use crate::{
    error::AppError,
    middleware::{X_CAPTCHA_TOKEN, api_key::has_valid_master_api_key},
    state::AppState,
};

#[derive(Debug, Deserialize)]
struct CaptchaProviderResponse {
    success: bool,
}

/// Middleware that verifies a CAPTCHA token unless a valid master API key is present.
pub async fn verify_captcha_token(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    if has_valid_master_api_key(req.headers(), state.config.as_ref()) {
        tracing::debug!("Bypassing captcha check due to valid x-api-key");
        return Ok(next.run(req).await);
    }

    let captcha_token = req
        .headers()
        .get(X_CAPTCHA_TOKEN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation("x-captcha-token header is required".to_string()))?;

    let secret_key = state
        .config
        .captcha_secret_key
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            AppError::ServiceUnavailable("CAPTCHA_SECRET_KEY is not configured".to_string())
        })?;

    let response = state
        .http_client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .timeout(std::time::Duration::from_secs(10))
        .form(&[("secret", secret_key), ("response", captcha_token)])
        .send()
        .await
        .map_err(|err| AppError::UpstreamError(format!("Failed to verify captcha: {err}")))?;

    let body = response
        .json::<CaptchaProviderResponse>()
        .await
        .map_err(|err| {
            AppError::UpstreamError(format!("Failed to parse captcha response: {err}"))
        })?;

    if !body.success {
        return Err(AppError::Validation("Invalid captcha token".to_string()));
    }

    Ok(next.run(req).await)
}
