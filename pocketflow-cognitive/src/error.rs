//! Error types for cognitive operations.

use thiserror::Error;

/// Errors that can occur during cognitive operations
#[derive(Error, Debug)]
pub enum CognitiveError {
    #[error("Reasoning failed: {message}")]
    ReasoningFailed { message: String },

    #[error("Planning failed: {message}")]
    PlanningFailed { message: String },

    #[error("Memory operation failed: {message}")]
    MemoryFailed { message: String },

    #[error("MCP communication error: {message}")]
    McpError { message: String },

    #[error("Invalid cognitive configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Context missing required data: {key}")]
    MissingContextData { key: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Core flow error: {0}")]
    CoreError(#[from] pocketflow_core::error::FlowError),

    #[error("MCP error: {0}")]
    Mcp(#[from] pocketflow_mcp::error::McpError),
}

/// Result type for cognitive operations
pub type Result<T> = std::result::Result<T, CognitiveError>;

impl From<CognitiveError> for pocketflow_core::error::FlowError {
    fn from(err: CognitiveError) -> Self {
        pocketflow_core::error::FlowError::context(err.to_string())
    }
}
