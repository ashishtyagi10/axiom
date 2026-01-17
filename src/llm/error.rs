//! LLM error types

use std::fmt;

/// Error type for LLM operations
#[derive(Debug, Clone)]
pub enum LlmError {
    /// Network/connection error
    Connection(String),

    /// API error (authentication, rate limit, etc.)
    Api { status: u16, message: String },

    /// Model not found
    ModelNotFound(String),

    /// Provider not available
    ProviderUnavailable(String),

    /// Request timeout
    Timeout,

    /// Invalid request (bad parameters)
    InvalidRequest(String),

    /// Rate limited
    RateLimited { retry_after: Option<u64> },

    /// Internal error
    Internal(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Connection(msg) => write!(f, "Connection error: {}", msg),
            LlmError::Api { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            LlmError::ModelNotFound(model) => write!(f, "Model not found: {}", model),
            LlmError::ProviderUnavailable(provider) => {
                write!(f, "Provider unavailable: {}", provider)
            }
            LlmError::Timeout => write!(f, "Request timed out"),
            LlmError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            LlmError::RateLimited { retry_after } => {
                if let Some(seconds) = retry_after {
                    write!(f, "Rate limited, retry after {} seconds", seconds)
                } else {
                    write!(f, "Rate limited")
                }
            }
            LlmError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for LlmError {}

impl From<ureq::Error> for LlmError {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::Status(status, response) => {
                let message = response.into_string().unwrap_or_else(|_| "Unknown error".to_string());
                if status == 429 {
                    LlmError::RateLimited { retry_after: None }
                } else if status == 401 || status == 403 {
                    LlmError::Api {
                        status,
                        message: "Authentication failed".to_string(),
                    }
                } else {
                    LlmError::Api { status, message }
                }
            }
            ureq::Error::Transport(transport) => LlmError::Connection(transport.to_string()),
        }
    }
}

impl From<std::io::Error> for LlmError {
    fn from(err: std::io::Error) -> Self {
        LlmError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for LlmError {
    fn from(err: serde_json::Error) -> Self {
        LlmError::Internal(format!("JSON error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_error_connection_display() {
        let err = LlmError::Connection("refused".to_string());
        assert_eq!(err.to_string(), "Connection error: refused");
    }

    #[test]
    fn test_llm_error_api_display() {
        let err = LlmError::Api {
            status: 401,
            message: "unauthorized".to_string(),
        };
        assert_eq!(err.to_string(), "API error (401): unauthorized");
    }

    #[test]
    fn test_llm_error_model_not_found() {
        let err = LlmError::ModelNotFound("gpt-5".to_string());
        assert_eq!(err.to_string(), "Model not found: gpt-5");
    }

    #[test]
    fn test_llm_error_provider_unavailable() {
        let err = LlmError::ProviderUnavailable("ollama".to_string());
        assert_eq!(err.to_string(), "Provider unavailable: ollama");
    }

    #[test]
    fn test_llm_error_timeout() {
        let err = LlmError::Timeout;
        assert_eq!(err.to_string(), "Request timed out");
    }

    #[test]
    fn test_llm_error_invalid_request() {
        let err = LlmError::InvalidRequest("bad params".to_string());
        assert_eq!(err.to_string(), "Invalid request: bad params");
    }

    #[test]
    fn test_llm_error_rate_limited_with_retry() {
        let err = LlmError::RateLimited { retry_after: Some(60) };
        assert_eq!(err.to_string(), "Rate limited, retry after 60 seconds");
    }

    #[test]
    fn test_llm_error_rate_limited_without_retry() {
        let err = LlmError::RateLimited { retry_after: None };
        assert_eq!(err.to_string(), "Rate limited");
    }

    #[test]
    fn test_llm_error_internal() {
        let err = LlmError::Internal("panic".to_string());
        assert_eq!(err.to_string(), "Internal error: panic");
    }

    #[test]
    fn test_llm_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let llm_err: LlmError = io_err.into();
        assert!(matches!(llm_err, LlmError::Internal(_)));
    }

    #[test]
    fn test_llm_error_debug() {
        let err = LlmError::Timeout;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Timeout"));
    }

    #[test]
    fn test_llm_error_clone() {
        let err = LlmError::Api {
            status: 500,
            message: "error".to_string(),
        };
        let cloned = err.clone();
        assert!(matches!(cloned, LlmError::Api { status: 500, .. }));
    }
}
