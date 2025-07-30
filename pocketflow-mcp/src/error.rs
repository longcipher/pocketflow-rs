//! Error types for MCP integration.

use thiserror::Error;

/// Errors that can occur during MCP operations.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("MCP connection failed: {message}")]
    ConnectionFailed {
        /// Error message
        message: String,
    },

    #[error("Tool '{tool_name}' not found")]
    ToolNotFound {
        /// Name of the tool that was not found
        tool_name: String,
    },

    #[error("Tool execution failed: {message}")]
    ToolExecutionFailed {
        /// Error message
        message: String,
    },

    #[error("Invalid tool arguments: {message}")]
    InvalidArguments {
        /// Error message
        message: String,
    },

    #[error("Resource '{uri}' not found")]
    ResourceNotFound {
        /// URI of the resource
        uri: String,
    },

    #[error("MCP server startup failed: {message}")]
    ServerStartupFailed { message: String },

    #[error("MCP client not found: {client_name}")]
    ClientNotFound { client_name: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MCP protocol error: {0}")]
    Protocol(String),
}

impl From<McpError> for pocketflow_core::error::FlowError {
    fn from(err: McpError) -> Self {
        pocketflow_core::error::FlowError::context(format!("MCP error: {err}"))
    }
}

impl From<pocketflow_core::error::FlowError> for McpError {
    fn from(err: pocketflow_core::error::FlowError) -> Self {
        McpError::Protocol(format!("Flow error: {err}"))
    }
}

pub type Result<T> = std::result::Result<T, McpError>;
