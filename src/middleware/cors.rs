use crate::{config::AppConfig, middleware::api_key::API_KEY_HEADER};
use axum::http::{
    Method,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderName},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use url::Url;

#[derive(Clone)]
enum OriginMatcher {
    Exact(String),
    Wildcard { prefix: String, suffix: String },
}

pub fn build_cors(config: &AppConfig) -> CorsLayer {
    let allowed_origins_env = config.allowed_origins.clone().unwrap_or_default();
    let raw_origins: Vec<String> = allowed_origins_env
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let default_methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
    ];

    if raw_origins.is_empty() || raw_origins == ["none"] {
        info!("CORS disabled (no origins allowed)");
        return CorsLayer::new()
            .allow_origin([])
            .allow_methods(default_methods);
    }

    if raw_origins.contains(&"*".to_string()) {
        info!("CORS enabled for all origins (wildcard)");
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(default_methods)
            .allow_headers(Any);
    }

    info!("CORS allowed origins: {:?}", raw_origins);

    // PRE-COMPUTE MATCHERS (Runs only once on startup)
    let mut matchers = Vec::new();
    for allowed in raw_origins {
        if allowed.contains('*') {
            let parts: Vec<&str> = allowed.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0].to_string();
                let suffix = parts[1].to_string();

                let is_safe = (prefix.ends_with("://") || prefix.ends_with('.'))
                    && (suffix.starts_with('.') || suffix.starts_with(':') || suffix.is_empty());

                if is_safe {
                    matchers.push(OriginMatcher::Wildcard { prefix, suffix });
                } else {
                    warn!("Unsafe wildcard pattern detected and ignored: {allowed}");
                }
            }
        } else {
            let allowed_base = Url::parse(&allowed)
                .map(|u| u.origin().ascii_serialization())
                .unwrap_or(allowed);
            matchers.push(OriginMatcher::Exact(allowed_base));
        }
    }

    let allow_origin = tower_http::cors::AllowOrigin::predicate(
        // CLOSURE (Runs on every request)
        move |origin: &axum::http::HeaderValue, _request_parts: &axum::http::request::Parts| {
            let Ok(origin_str) = origin.to_str() else {
                return false;
            };
            let Ok(origin_url) = Url::parse(origin_str) else {
                return false;
            };
            let origin_base = origin_url.origin().ascii_serialization();

            for matcher in &matchers {
                match matcher {
                    OriginMatcher::Exact(exact) => {
                        if &origin_base == exact {
                            return true;
                        }
                    }
                    OriginMatcher::Wildcard { prefix, suffix } => {
                        if origin_base.starts_with(prefix) && origin_base.ends_with(suffix) {
                            return true;
                        }
                    }
                }
            }
            false
        },
    );

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods(default_methods)
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            ACCEPT,
            HeaderName::from_static(API_KEY_HEADER),
            HeaderName::from_static("x-captcha-token"),
        ])
        .allow_credentials(true)
}
