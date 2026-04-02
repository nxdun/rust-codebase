use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::NonZeroU32,
    sync::Arc,
};

use axum::{
    Json,
    extract::{Request, State, connect_info::ConnectInfo},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{
    Quota, RateLimiter, clock::DefaultClock, middleware::NoOpMiddleware,
    state::keyed::DefaultKeyedStateStore,
};
use serde_json::json;
use tracing::{debug, info};

use crate::{config::AppConfig, middleware::api_key::has_valid_master_api_key, state::AppState};

const RATE_LIMITER_PER_SECOND: u32 = 10;
const RATE_LIMITER_BURST_SIZE: u32 = 20;

const ENHANCED_RATE_LIMITER_PER_SECOND: u32 = 50;
const ENHANCED_RATE_LIMITER_BURST_SIZE: u32 = 100;

pub const TIER_NORMAL: &str = "normal";
pub const TIER_ENHANCED: &str = "enhanced";

type KeyedLimiter =
    RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock, NoOpMiddleware>;

#[derive(Clone)]
pub struct RateLimiters {
    normal: Arc<KeyedLimiter>,
    enhanced: Arc<KeyedLimiter>,
}

impl RateLimiters {
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

    fn get_limiter_and_tier(&self, has_valid_api_key: bool) -> (&KeyedLimiter, &'static str) {
        if has_valid_api_key {
            (self.enhanced.as_ref(), TIER_ENHANCED)
        } else {
            (self.normal.as_ref(), TIER_NORMAL)
        }
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

fn build_limiter(per_second: u32, burst_size: u32) -> KeyedLimiter {
    let quota = Quota::per_second(NonZeroU32::new(per_second).expect("per_second must be > 0"))
        .allow_burst(NonZeroU32::new(burst_size).expect("burst_size must be > 0"));
    RateLimiter::<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock, NoOpMiddleware>::dashmap(
        quota,
    )
}

pub fn is_production(config: &AppConfig) -> bool {
    config.env == "production"
}

fn is_rate_limit_exempt(req: &Request) -> bool {
    req.uri().path() == "/health"
}

fn request_client_ip(req: &Request, config: &AppConfig) -> IpAddr {
    if !is_production(config) {
        return IpAddr::V4(Ipv4Addr::LOCALHOST);
    }

    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

pub async fn enforce_tiered_rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if req.method() == Method::OPTIONS || is_rate_limit_exempt(&req) {
        return Ok(next.run(req).await);
    }

    let has_valid_api_key = has_valid_master_api_key(req.headers(), state.config.as_ref());
    let client_ip = request_client_ip(&req, state.config.as_ref());

    let (limiter, tier) = state.rate_limiters.get_limiter_and_tier(has_valid_api_key);

    if limiter.check_key(&client_ip).is_err() {
        debug!(
            client_ip = %client_ip,
            tier = tier,
            "request rejected by rate limiter"
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "message": "Rate limit exceeded", "tier": tier })),
        ));
    }

    Ok(next.run(req).await)
}

pub fn log_rate_limit_mode(config: &AppConfig) {
    let mode_desc = if is_production(config) {
        "production mode (keyed by client IP)"
    } else {
        "development mode (global key)"
    };

    info!(
        "Rate Limiter: {}, {}={}/s burst={}, {}={}/s burst={}",
        mode_desc,
        TIER_NORMAL,
        RATE_LIMITER_PER_SECOND,
        RATE_LIMITER_BURST_SIZE,
        TIER_ENHANCED,
        ENHANCED_RATE_LIMITER_PER_SECOND,
        ENHANCED_RATE_LIMITER_BURST_SIZE
    );
}
