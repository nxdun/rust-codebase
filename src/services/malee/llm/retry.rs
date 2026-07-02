use crate::error::MaleeError;

/// Classifies an LLM/stream error to decide the retry strategy.
///
/// Replaces fragile `err_str.contains("RATE_LIMIT")` checks with typed
/// pattern matching on `MaleeError` variants and known error string patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Rate limited by provider — rotate backend + backoff.
    RateLimited,
    /// Transient server error (5xx) — rotate backend.
    TransientServer,
    /// Malformed LLM output (bad JSON in tool args) — retry same backend.
    MalformedOutput,
    /// Network/timeout — retry same backend with backoff.
    NetworkError,
    /// Client or auth error (4xx non-429) or unknown — do NOT retry.
    Fatal,
}

impl ErrorClass {
    /// Classifies a `MaleeError` into a retry category.
    ///
    /// For `LlmError` and `ConnectorError`, inspects the message string
    /// for known provider error patterns. Structured variants like
    /// `LlmRateLimited` and `LlmStreamTimeout` are matched directly.
    pub fn classify(err: &MaleeError) -> Self {
        match err {
            MaleeError::LlmRateLimited { .. } => Self::RateLimited,
            MaleeError::LlmStreamTimeout { .. } => Self::NetworkError,
            MaleeError::LlmMalformedOutput(_) => Self::MalformedOutput,
            MaleeError::LlmError(msg) | MaleeError::ConnectorError(msg) => {
                Self::classify_message(msg)
            }
            // All other MaleeError variants are non-retryable domain errors
            _ => Self::Fatal,
        }
    }

    /// Inspect an error message string for known retryable patterns.
    fn classify_message(msg: &str) -> Self {
        // Rate limiting indicators
        if msg.contains("RATE_LIMIT")
            || msg.contains("429")
            || msg.contains("Too Many Requests")
            || msg.contains("rate_limit_exceeded")
        {
            return Self::RateLimited;
        }

        // Transient server errors
        if msg.contains("HTTP 5")
            || msg.contains("502")
            || msg.contains("503")
            || msg.contains("504")
            || msg.contains("tool_use_failed")
        {
            return Self::TransientServer;
        }

        // Network-level failures
        if msg.contains("timeout") || msg.contains("connection") || msg.contains("Stream timeout") {
            return Self::NetworkError;
        }

        // HTTP 400 can sometimes be a transient tool_use issue
        if msg.contains("HTTP 400") {
            return Self::TransientServer;
        }

        Self::Fatal
    }

    /// Whether this error class should trigger a retry at all.
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::TransientServer | Self::NetworkError
        )
    }

    /// Whether the retry should rotate to a different backend.
    pub const fn should_rotate_backend(self) -> bool {
        matches!(self, Self::RateLimited | Self::TransientServer)
    }
}
