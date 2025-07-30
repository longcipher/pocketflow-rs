use std::fmt;

use pocketflow_core::error::FlowError;
use thiserror::Error;

/// Result type for tool operations
pub type Result<T> = std::result::Result<T, ToolError>;

/// Main error type for tool operations
#[derive(Error, Debug, Clone)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Invalid field '{field}': {message}")]
    InvalidField { field: String, message: String },

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("External service error: {category}: {message}")]
    ExternalService { category: String, message: String },

    #[error("Resource unavailable: {0}")]
    ResourceUnavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ToolError {
    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    /// Create an invalid parameters error
    pub fn invalid_parameters(message: impl Into<String>) -> Self {
        Self::InvalidParameters(message.into())
    }

    /// Create an invalid field error
    pub fn invalid_field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidField {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an execution error
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution(message.into())
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(message.into())
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout(message.into())
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication(message.into())
    }

    /// Create a permission error
    pub fn permission(message: impl Into<String>) -> Self {
        Self::Permission(message.into())
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::RateLimit(message.into())
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into())
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Create an IO error
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }

    /// Create an external service error
    pub fn external_service(category: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            category: category.into(),
            message: message.into(),
        }
    }

    /// Create a resource unavailable error
    pub fn resource_unavailable(message: impl Into<String>) -> Self {
        Self::ResourceUnavailable(message.into())
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Get the error category for classification
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::NotFound(_) => ErrorCategory::NotFound,
            Self::InvalidParameters(_) | Self::InvalidField { .. } => ErrorCategory::InvalidInput,
            Self::Execution(_) => ErrorCategory::Execution,
            Self::Network(_) => ErrorCategory::Network,
            Self::Timeout(_) => ErrorCategory::Timeout,
            Self::Authentication(_) => ErrorCategory::Authentication,
            Self::Permission(_) => ErrorCategory::Permission,
            Self::RateLimit(_) => ErrorCategory::RateLimit,
            Self::Validation(_) => ErrorCategory::Validation,
            Self::Serialization(_) => ErrorCategory::Serialization,
            Self::Configuration(_) => ErrorCategory::Configuration,
            Self::Io(_) => ErrorCategory::Io,
            Self::ExternalService { .. } => ErrorCategory::ExternalService,
            Self::ResourceUnavailable(_) => ErrorCategory::ResourceUnavailable,
            Self::Internal(_) => ErrorCategory::Internal,
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::Network
                | ErrorCategory::Timeout
                | ErrorCategory::RateLimit
                | ErrorCategory::ResourceUnavailable
        )
    }

    /// Check if the error is a user error (not a system error)
    pub fn is_user_error(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::InvalidInput
                | ErrorCategory::Authentication
                | ErrorCategory::Permission
                | ErrorCategory::NotFound
        )
    }
}

/// Error categories for classification and handling
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    NotFound,
    InvalidInput,
    Execution,
    Network,
    Timeout,
    Authentication,
    Permission,
    RateLimit,
    Validation,
    Serialization,
    Configuration,
    Io,
    ExternalService,
    ResourceUnavailable,
    Internal,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::NotFound => write!(f, "not_found"),
            ErrorCategory::InvalidInput => write!(f, "invalid_input"),
            ErrorCategory::Execution => write!(f, "execution"),
            ErrorCategory::Network => write!(f, "network"),
            ErrorCategory::Timeout => write!(f, "timeout"),
            ErrorCategory::Authentication => write!(f, "authentication"),
            ErrorCategory::Permission => write!(f, "permission"),
            ErrorCategory::RateLimit => write!(f, "rate_limit"),
            ErrorCategory::Validation => write!(f, "validation"),
            ErrorCategory::Serialization => write!(f, "serialization"),
            ErrorCategory::Configuration => write!(f, "configuration"),
            ErrorCategory::Io => write!(f, "io"),
            ErrorCategory::ExternalService => write!(f, "external_service"),
            ErrorCategory::ResourceUnavailable => write!(f, "resource_unavailable"),
            ErrorCategory::Internal => write!(f, "internal"),
        }
    }
}

// Standard library integrations
impl From<std::io::Error> for ToolError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for ToolError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for ToolError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout(err.to_string())
        } else if err.is_connect() {
            Self::Network(err.to_string())
        } else if err.is_status() {
            let status = err.status().map(|s| s.as_u16()).unwrap_or(0);
            match status {
                401 => Self::Authentication(err.to_string()),
                403 => Self::Permission(err.to_string()),
                404 => Self::NotFound(err.to_string()),
                429 => Self::RateLimit(err.to_string()),
                500..=599 => Self::ExternalService {
                    category: "server_error".to_string(),
                    message: err.to_string(),
                },
                _ => Self::Network(err.to_string()),
            }
        } else {
            Self::Network(err.to_string())
        }
    }
}

// Integration with pocketflow-core errors
impl From<ToolError> for FlowError {
    fn from(err: ToolError) -> Self {
        match err.category() {
            ErrorCategory::InvalidInput | ErrorCategory::Validation => {
                FlowError::construction(err.to_string())
            }
            ErrorCategory::Configuration => FlowError::construction(err.to_string()),
            _ => FlowError::context(err.to_string()),
        }
    }
}

impl From<FlowError> for ToolError {
    fn from(err: FlowError) -> Self {
        Self::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ToolError::not_found("test tool");
        assert_eq!(err.category(), ErrorCategory::NotFound);
        assert!(!err.is_retryable());
        assert!(err.is_user_error());
    }

    #[test]
    fn test_error_retryable() {
        let network_err = ToolError::network("connection failed");
        assert!(network_err.is_retryable());
        assert!(!network_err.is_user_error());

        let invalid_err = ToolError::invalid_parameters("bad params");
        assert!(!invalid_err.is_retryable());
        assert!(invalid_err.is_user_error());
    }
}
