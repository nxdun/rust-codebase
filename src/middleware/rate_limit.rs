use std::{net::SocketAddr, num::NonZeroU32, sync::Arc};

use axum::{
    extract::{Request, State, connect_info::ConnectInfo},
    http::Method,
    middleware::Next,
    response::Response,
};
use governor::{
    Quota, RateLimiter, clock::DefaultClock, middleware::NoOpMiddleware,
    state::keyed::DefaultKeyedStateStore,
};
use tracing::{debug, info};

use crate::{
    config::AppConfig, error::AppError, middleware::api_key::has_valid_master_api_key,
    state::AppState,
};

const RATE_LIMITER_PER_SECOND: u32 = 10;
const RATE_LIMITER_BURST_SIZE: u32 = 20;

const ENHANCED_RATE_LIMITER_PER_SECOND: u32 = 50;
const ENHANCED_RATE_LIMITER_BURST_SIZE: u32 = 100;

// Checkpoint
const _: () = {
    assert!(
        RATE_LIMITER_PER_SECOND > 0
            && RATE_LIMITER_BURST_SIZE > 0
            && ENHANCED_RATE_LIMITER_PER_SECOND > 0
            && ENHANCED_RATE_LIMITER_BURST_SIZE > 0,
        "Rate limit constants must be positive"
    );
    assert!(
        RATE_LIMITER_PER_SECOND <= ENHANCED_RATE_LIMITER_PER_SECOND,
        "Normal rate limit cannot exceed enhanced rate limit"
    );
    assert!(
        RATE_LIMITER_BURST_SIZE <= ENHANCED_RATE_LIMITER_BURST_SIZE,
        "Normal burst size cannot exceed enhanced burst size"
    );
};

type KeyedLimiter =
    RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock, NoOpMiddleware>;

/// Implements tiered rate limiting. request ip based.
/// enhanced: requests only with correct API key.
/// normal: all other requests.
#[derive(Clone, Debug)]
pub struct RateLimiters {
    normal: Arc<KeyedLimiter>,
    enhanced: Arc<KeyedLimiter>,
}

impl RateLimiters {
    /// Creates a new instance of `RateLimiters`
    ///
    /// Panics if rate limit constants above are invalid.
    #[must_use]
    pub fn new() -> Self {
        Self {
            normal: Arc::new(build_limiter(
                RATE_LIMITER_PER_SECOND,
                RATE_LIMITER_BURST_SIZE,
            )),
            enhanced: Arc::new(build_limiter(
                ENHANCED_RATE_LIMITER_PER_SECOND,
                ENHANCED_RATE_LIMITER_BURST_SIZE,
            )),
        }
    }

    fn limiter_for_api_key(&self, has_valid_api_key: bool) -> &KeyedLimiter {
        if has_valid_api_key {
            self.enhanced.as_ref()
        } else {
            self.normal.as_ref()
        }
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal helper to build a DashMap-backed rate limiter.
#[allow(clippy::expect_used)]
fn build_limiter(per_second: u32, burst_size: u32) -> KeyedLimiter {
    // Safety: The constants are validated at compile time by the assertions above, so these unwraps will never panic.
    let per_second = NonZeroU32::new(per_second).expect("RATE_LIMITER_PER_SECOND must be positive");
    let burst_size = NonZeroU32::new(burst_size).expect("RATE_LIMITER_BURST_SIZE must be positive");

    let quota = Quota::per_second(per_second).allow_burst(burst_size);
    RateLimiter::<String, DefaultKeyedStateStore<String>, DefaultClock, NoOpMiddleware>::dashmap(
        quota,
    )
}

/// Helper to check if the app is running in production mode.
#[must_use]
pub fn is_production(config: &AppConfig) -> bool {
    config.env == "production"
}

fn is_rate_limit_exempt(req: &Request) -> bool {
    req.uri().path() == "/health"
}

fn request_client_key(req: &Request, config: &AppConfig) -> String {
    if !is_production(config) {
        return "global".to_string();
    }

    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(
            || "unknown-client".to_string(),
            |connect_info| connect_info.0.ip().to_string(),
        )
}

/// Middleware that enforces tiered rate limits based on API key presence and client IP.
///
/// Rejects requests exceeding the quota with a `403 Forbidden` error.
pub async fn enforce_tiered_rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if req.method() == Method::OPTIONS || is_rate_limit_exempt(&req) {
        return Ok(next.run(req).await);
    }

    let has_valid_api_key = has_valid_master_api_key(req.headers(), state.config.as_ref());
    let client_key = request_client_key(&req, state.config.as_ref());
    let limiter = state.rate_limiters.limiter_for_api_key(has_valid_api_key);

    if limiter.check_key(&client_key).is_err() {
        let tier = if has_valid_api_key {
            "enhanced"
        } else {
            "normal"
        };
        debug!(
            client_key = %client_key,
            tier = tier,
            "request rejected by rate limiter"
        );
        return Err(AppError::Forbidden(format!("Rate limit exceeded ({tier})")));
    }

    Ok(next.run(req).await)
}

/// Logs the current rate limiting configuration to the tracing output.
pub fn log_rate_limit_mode(config: &AppConfig) {
    let mode = if is_production(config) {
        "production"
    } else {
        "development"
    };
    info!(
        "Rate Limiter: {} mode, normal={}/s burst={}, enhanced={}/s burst={}",
        mode,
        RATE_LIMITER_PER_SECOND,
        RATE_LIMITER_BURST_SIZE,
        ENHANCED_RATE_LIMITER_PER_SECOND,
        ENHANCED_RATE_LIMITER_BURST_SIZE
    );
}
