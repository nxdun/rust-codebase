#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "api/auth_tests.rs"]
mod auth_tests;
#[path = "api/captcha_tests.rs"]
mod captcha_tests;
#[path = "api/common.rs"]
mod common;
#[path = "api/cors_tests.rs"]
mod cors_tests;
#[path = "api/health_tests.rs"]
mod health_tests;
#[path = "api/rate_limit_tests.rs"]
mod rate_limit_tests;
#[path = "api/root_tests.rs"]
mod root_tests;
#[path = "api/routing_tests.rs"]
mod routing_tests;
#[path = "api/validation_tests.rs"]
mod validation_tests;
#[path = "api/ytdlp_tests.rs"]
mod ytdlp_tests;
