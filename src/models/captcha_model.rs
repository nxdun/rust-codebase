use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CaptchaVerifyRequest {
    pub captcha: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCaptchaVerifyResponse {
    pub success: bool,
}
