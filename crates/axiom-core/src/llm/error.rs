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
