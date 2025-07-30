//! Error types for PocketFlow-RS.

use thiserror::Error;

/// Result type for flow operations.
pub type Result<T> = std::result::Result<T, FlowError>;

/// Error types that can occur during flow execution.
#[derive(Error, Debug)]
pub enum FlowError {
    /// Context manipulation error.
    #[error("Context error: {0}")]
    Context(String),

    /// Flow construction error.
    #[error("Construction error: {0}")]
    Construction(String),

    /// Invalid state transition.
    #[error("Invalid transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// Source state
        from: String,
        /// Target state
        to: String,
    },

    /// Serialization/Deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error.
    #[error("Error: {0}")]
    Generic(#[from] eyre::Report),

    /// Flow execution was cancelled.
    #[error("Flow execution was cancelled")]
    Cancelled,

    /// Flow execution timeout.
    #[error("Flow execution timed out")]
    Timeout,
}

impl FlowError {
    /// Create a new context error.
    pub fn context(msg: impl Into<String>) -> Self {
        Self::Context(msg.into())
    }

    /// Create a new construction error.
    pub fn construction(msg: impl Into<String>) -> Self {
        Self::Construction(msg.into())
    }

    /// Create an execution error.
    pub fn execution(msg: impl Into<String>) -> Self {
        Self::Context(msg.into())
    }

    /// Create a new invalid transition error.
    pub fn invalid_transition(from: impl std::fmt::Debug, to: impl std::fmt::Debug) -> Self {
        Self::InvalidTransition {
            from: format!("{from:?}"),
            to: format!("{to:?}"),
        }
    }
}
