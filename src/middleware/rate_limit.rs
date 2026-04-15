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

type KeyedLimiter =
    RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock, NoOpMiddleware>;

#[derive(Clone, Debug)]
pub struct RateLimiters {
    normal: Arc<KeyedLimiter>,
    enhanced: Arc<KeyedLimiter>,
}

impl RateLimiters {
    /// Creates a new instance of `RateLimiters` with normal and enhanced buckets.
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

fn build_limiter(per_second: u32, burst_size: u32) -> KeyedLimiter {
    let quota = Quota::per_second(
        NonZeroU32::new(per_second).expect("rate limiter per_second must be greater than 0"),
    )
    .allow_burst(
        NonZeroU32::new(burst_size).expect("rate limiter burst_size must be greater than 0"),
    );
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

pub fn log_rate_limit_mode(config: &AppConfig) {
    if is_production(config) {
        info!(
            "Rate Limiter: production mode (keyed by client IP), normal={}/s burst={}, enhanced={}/s burst={}",
            RATE_LIMITER_PER_SECOND,
            RATE_LIMITER_BURST_SIZE,
            ENHANCED_RATE_LIMITER_PER_SECOND,
            ENHANCED_RATE_LIMITER_BURST_SIZE
        );
    } else {
        info!(
            "Rate Limiter: development mode (global key), normal={}/s burst={}, enhanced={}/s burst={}",
            RATE_LIMITER_PER_SECOND,
            RATE_LIMITER_BURST_SIZE,
            ENHANCED_RATE_LIMITER_PER_SECOND,
            ENHANCED_RATE_LIMITER_BURST_SIZE
        );
    }
}
