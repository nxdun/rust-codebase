use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CaptchaVerifyRequest {
    pub captcha: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GoogleCaptchaVerifyResponse {
    pub success: bool,
    // for furure use, currently not used
    pub challenge_ts: Option<String>,
    pub hostname: Option<String>,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
    pub score: Option<f64>,
}
