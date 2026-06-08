use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
};
use metrics::{counter, histogram};
use serde::Deserialize;
use std::time::Instant;

use shared_core::error::AppError;
use crate::{
    middleware::{HEADER_CAPTCHA_NAME, api_key::has_valid_master_api_key},
    state::AppState,
};

const CAPTCHA_VERIFY_TIMEOUT_SECS: u64 = 10;

// Checkpoint
const _: () = {
    assert!(
        CAPTCHA_VERIFY_TIMEOUT_SECS > 0,
        "CAPTCHA_VERIFY_TIMEOUT_SECS must be positive"
    );
    assert!(
        CAPTCHA_VERIFY_TIMEOUT_SECS <= 60,
        "CAPTCHA_VERIFY_TIMEOUT_SECS is unusually high (> 1min)"
    );
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
        counter!("captcha_check_total", "status" => "bypass").increment(1);
        return Ok(next.run(req).await);
    }

    let captcha_token = req
        .headers()
        .get(HEADER_CAPTCHA_NAME)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if captcha_token.is_none() {
        counter!("captcha_check_total", "status" => "failure", "reason" => "missing").increment(1);
        return Err(AppError::Validation(
            "x-captcha-token header is required".to_string(),
        ));
    }
    let captcha_token = captcha_token.unwrap_or_default();

    let secret_key = state
        .config
        .captcha_secret_key()
        .filter(|s| !s.trim().is_empty());

    if secret_key.is_none() {
        counter!("captcha_check_total", "status" => "error", "reason" => "config").increment(1);
        return Err(AppError::ServiceUnavailable(
            "CAPTCHA_SECRET_KEY is not configured".to_string(),
        ));
    }
    let secret_key = secret_key.unwrap_or_default();

    let start = Instant::now();
    let response = state
        .http_client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .timeout(std::time::Duration::from_secs(CAPTCHA_VERIFY_TIMEOUT_SECS))
        .form(&[("secret", secret_key), ("response", captcha_token)])
        .send()
        .await;

    let latency = start.elapsed();
    histogram!("captcha_verify_duration_seconds").record(latency.as_secs_f64());

    let response = response.map_err(|err| {
        counter!("captcha_check_total", "status" => "failure", "reason" => "upstream_error")
            .increment(1);
        AppError::UpstreamError(format!("Failed to verify captcha: {err}"))
    })?;

    let body = response
        .json::<CaptchaProviderResponse>()
        .await
        .map_err(|err| {
            counter!("captcha_check_total", "status" => "failure", "reason" => "parse_error")
                .increment(1);
            AppError::UpstreamError(format!("Failed to parse captcha response: {err}"))
        })?;

    if !body.success {
        counter!("captcha_check_total", "status" => "failure", "reason" => "invalid").increment(1);
        return Err(AppError::Validation("Invalid captcha token".to_string()));
    }

    counter!("captcha_check_total", "status" => "success").increment(1);
    Ok(next.run(req).await)
}
