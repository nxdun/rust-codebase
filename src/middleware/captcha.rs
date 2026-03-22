use axum::{
    Json,
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::Deserialize;
use serde_json::json;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct CaptchaProviderResponse {
    success: bool,
}

pub async fn verify_captcha_token(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    // critical: If x-bypass-dev header is set to 'true' as a string, skip captcha verification :}
    // if req
    //     .headers()
    //     .get("x-bypass-dev")
    //     .and_then(|value| value.to_str().ok())
    //     .map(str::trim)
    //     .map(|v| v.eq_ignore_ascii_case("true"))
    //     .unwrap_or(false)
    // {
    //     tracing::warn!("Captcha verification BYPASSED due to x-bypass-dev header");
    //     return Ok(next.run(req).await);
    // }

    let captcha_token = match req
        .headers()
        .get("x-captcha-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "message": "x-captcha-token header is required" })),
            ));
        }
    };

    let secret_key = match state.config.captcha_secret_key.as_deref() {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "CAPTCHA_SECRET_KEY is not configured" })),
            ));
        }
    };

    let response = match state
        .http_client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .timeout(std::time::Duration::from_secs(10))
        .form(&[("secret", secret_key), ("response", captcha_token)])
        .send()
        .await
    {
        Ok(result) => result,
        Err(err) => {
            tracing::error!("Failed to send captcha verification request: {}", err);
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(json!({ "message": "Failed to verify captcha token" })),
            ));
        }
    };

    let body = match response.json::<CaptchaProviderResponse>().await {
        Ok(parsed) => parsed,
        Err(err) => {
            tracing::error!("Failed to parse captcha provider response: {}", err);
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(json!({ "message": "Failed to parse captcha provider response" })),
            ));
        }
    };

    if !body.success {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "message": "Invalid captcha token" })),
        ));
    }

    Ok(next.run(req).await)
}
