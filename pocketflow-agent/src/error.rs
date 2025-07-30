//! Error types for PocketFlow agent operations.

use pocketflow_core::prelude::FlowError;
use thiserror::Error;

/// Result type for agent operations.
pub type Result<T> = std::result::Result<T, AgentError>;

/// Main error type for agent operations.
#[derive(Error, Debug, Clone)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Tool execution error: {tool}: {message}")]
    ToolExecution { tool: String, message: String },

    #[error("Agent delegation error: {target}: {message}")]
    Delegation { target: String, message: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Rate limit error: {0}")]
    RateLimit(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("State transition error: from {from} to {to}: {reason}")]
    StateTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Streaming error: {0}")]
    Streaming(String),

    #[error("Coordination error: {0}")]
    Coordination(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AgentError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Create a model error
    pub fn model(message: impl Into<String>) -> Self {
        Self::Model(message.into())
    }

    /// Create a tool execution error
    pub fn tool_execution(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ToolExecution {
            tool: tool.into(),
            message: message.into(),
        }
    }

    /// Create a delegation error
    pub fn delegation(target: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Delegation {
            target: target.into(),
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout(message.into())
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::RateLimit(message.into())
    }

    /// Create a permission error
    pub fn permission(message: impl Into<String>) -> Self {
        Self::Permission(message.into())
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    /// Create a state transition error
    pub fn state_transition(
        from: impl Into<String>,
        to: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::StateTransition {
            from: from.into(),
            to: to.into(),
            reason: reason.into(),
        }
    }

    /// Create a streaming error
    pub fn streaming(message: impl Into<String>) -> Self {
        Self::Streaming(message.into())
    }

    /// Create a coordination error
    pub fn coordination(message: impl Into<String>) -> Self {
        Self::Coordination(message.into())
    }

    /// Create a context error
    pub fn context(message: impl Into<String>) -> Self {
        Self::Context(message.into())
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into())
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout(_) | Self::RateLimit(_) | Self::Model(_))
    }

    /// Check if the error is a user error
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            Self::Configuration(_) | Self::Validation(_) | Self::Permission(_) | Self::NotFound(_)
        )
    }

    /// Get error category for logging/monitoring
    pub fn category(&self) -> &'static str {
        match self {
            Self::Configuration(_) => "configuration",
            Self::Model(_) => "model",
            Self::ToolExecution { .. } => "tool_execution",
            Self::Delegation { .. } => "delegation",
            Self::Validation(_) => "validation",
            Self::Timeout(_) => "timeout",
            Self::RateLimit(_) => "rate_limit",
            Self::Permission(_) => "permission",
            Self::NotFound(_) => "not_found",
            Self::StateTransition { .. } => "state_transition",
            Self::Streaming(_) => "streaming",
            Self::Coordination(_) => "coordination",
            Self::Context(_) => "context",
            Self::Serialization(_) => "serialization",
            Self::Internal(_) => "internal",
        }
    }
}

// Standard library integrations
impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<genai::Error> for AgentError {
    fn from(err: genai::Error) -> Self {
        Self::Model(err.to_string())
    }
}

// Integration with pocketflow-core
impl From<AgentError> for FlowError {
    fn from(err: AgentError) -> Self {
        match &err {
            AgentError::Configuration(_) | AgentError::Validation(_) => {
                FlowError::construction(err.to_string())
            }
            _ => FlowError::context(err.to_string()),
        }
    }
}

impl From<FlowError> for AgentError {
    fn from(err: FlowError) -> Self {
        Self::Context(err.to_string())
    }
}

// Integration with pocketflow-tools
impl From<pocketflow_tools::ToolError> for AgentError {
    fn from(err: pocketflow_tools::ToolError) -> Self {
        Self::ToolExecution {
            tool: "unknown".to_string(),
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = AgentError::model("test model error");
        assert_eq!(err.category(), "model");
        assert!(err.is_retryable());
        assert!(!err.is_user_error());
    }

    #[test]
    fn test_error_conversion() {
        let json_err = serde_json::from_str::<i32>("invalid").unwrap_err();
        let agent_err = AgentError::from(json_err);
        assert_eq!(agent_err.category(), "serialization");
    }
}
