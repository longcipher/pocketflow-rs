//! PocketFlow Tools - A comprehensive tool system for workflow automation
//!
//! This crate provides a unified interface for tools that can be executed within
//! PocketFlow workflows. It includes:
//!
//! - A robust tool abstraction with parameter validation
//! - Tool registry for discovery and execution
//! - Built-in utilities for common operations
//! - Integration with pocketflow-core and other ecosystem crates
//!
//! ## Example Usage
//!
//! ```rust
//! use pocketflow_tools::prelude::*;
//!
//! // Define a custom tool
//! struct EchoTool;
//!
//! #[async_trait]
//! impl Tool for EchoTool {
//!     fn name(&self) -> &str { "echo" }
//!     fn description(&self) -> &str { "Echo input back" }
//!     fn category(&self) -> ToolCategory { ToolCategory::System }
//!     
//!     fn parameter_schema(&self) -> serde_json::Value {
//!         ToolParameters::new_schema()
//!             .add_required("message", "string", "Message to echo")
//!             .into()
//!     }
//!     
//!     async fn execute(&self, params: ToolParameters, _ctx: ToolContext) -> Result<ToolResult> {
//!         let message: String = params.get("message")?;
//!         Ok(ToolResult::success(message))
//!     }
//! }
//!
//! // Use in a registry
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut registry = ToolRegistry::new();
//!     registry.register_tool(Box::new(EchoTool)).await?;
//!     
//!     let context = ToolContext::new();
//!     let params = serde_json::json!({"message": "Hello, World!"});
//!     
//!     let result = registry.execute_tool("echo", &params, &context).await?;
//!     println!("Result: {}", result.content);
//!     
//!     Ok(())
//! }
//! ```

/// Core tool abstractions and trait definitions.
pub mod core;
/// Custom tool implementations and helpers.
pub mod custom;
/// Error types for tool operations.
pub mod error;
/// Python execution tool implementation.
pub mod python;
/// Tool registry for discovery and execution.
pub mod registry;
/// Utility functions for tool development.
pub mod utils;
/// Parameter validation utilities.
pub mod validation;
/// Web search/fetch tool implementation.
pub mod web;

// Re-export commonly used types
pub use core::{
    CacheConfig, ContentType, RateLimit, RetryConfig, Tool, ToolCapability, ToolCategory,
    ToolContext, ToolInfo, ToolMetadata, ToolParameters, ToolResult,
};

pub use error::{ErrorCategory, Result, ToolError};
pub use registry::{ToolComposition, ToolRegistry, ToolRequest};

/// Prelude module for convenient imports
pub mod prelude {
    pub use async_trait::async_trait;
    pub use serde_json::{Value, json};
    pub use uuid::Uuid;

    pub use crate::{
        core::{
            ContentType, Tool, ToolCapability, ToolCategory, ToolContext, ToolInfo, ToolMetadata,
            ToolParameters, ToolResult,
        },
        error::{ErrorCategory, Result, ToolError},
        python::PythonExecutionTool,
        registry::{ToolComposition, ToolRegistry, ToolRequest},
        utils::{conversion_utils, perf_utils, template_utils, tool_utils},
        web::WebSearchTool,
    };
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn category(&self) -> ToolCategory {
            ToolCategory::Custom
        }

        fn parameter_schema(&self) -> serde_json::Value {
            ToolParameters::new_schema()
                .add_required("input", "string", "Test input")
                .into()
        }

        async fn execute(&self, params: ToolParameters, _ctx: ToolContext) -> Result<ToolResult> {
            let input: String = params.get("input")?;
            Ok(ToolResult::success(format!("Processed: {input}")))
        }
    }

    #[tokio::test]
    async fn test_basic_tool_execution() {
        let tool = TestTool;
        let params = ToolParameters::new(json!({"input": "test data"}));
        let context = ToolContext::new();

        let result = tool.execute(params, context).await.unwrap();
        assert!(result.is_success());
        assert_eq!(result.content, "Processed: test data");
    }

    #[tokio::test]
    async fn test_registry_integration() {
        let mut registry = ToolRegistry::new();
        registry.register_tool(Box::new(TestTool)).await.unwrap();

        let tools = registry.list_tools().await;
        assert!(tools.contains(&"test".to_string()));

        let context = ToolContext::new();
        let params = json!({"input": "registry test"});

        let result = registry
            .execute_tool("test", &params, &context)
            .await
            .unwrap();
        assert!(result.is_success());
        assert!(result.content.contains("registry test"));
    }
}
