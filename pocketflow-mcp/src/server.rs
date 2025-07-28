//! MCP server implementation for workflow integration.

use std::{collections::HashMap, sync::Arc};

use pocketflow_core::{context::Context, node::Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use super::{Resource, Result, Tool, error::McpError};

/// Handler for exposing workflow functionality as MCP tools.
#[derive(Debug, Clone)]
pub struct WorkflowMcpHandler {
    /// Available MCP tools.
    pub tools: Arc<RwLock<HashMap<String, Tool>>>,
    /// Available MCP resources.
    pub resources: Arc<RwLock<HashMap<String, Resource>>>,
    /// Workflow context.
    pub context: Arc<RwLock<Context>>,
}

impl WorkflowMcpHandler {
    /// Create a new MCP handler.
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(HashMap::new())),
            context: Arc::new(RwLock::new(Context::default())),
        }
    }

    /// Register a tool.
    pub async fn register_tool(&self, name: String, tool: Tool) {
        self.tools.write().await.insert(name, tool);
    }

    /// Register a resource.
    pub async fn register_resource(&self, name: String, resource: Resource) {
        self.resources.write().await.insert(name, resource);
    }

    /// List available tools.
    pub async fn list_tools(&self) -> Vec<Tool> {
        self.tools.read().await.values().cloned().collect()
    }

    /// List available resources.
    pub async fn list_resources(&self) -> Vec<Resource> {
        self.resources.read().await.values().cloned().collect()
    }
}

impl Default for WorkflowMcpHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for workflow execution via MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionParams {
    /// Workflow name to execute.
    pub workflow_name: String,
    /// Input data for the workflow.
    pub input: Value,
    /// Optional context overrides.
    pub context_overrides: Option<HashMap<String, Value>>,
}

/// Result of workflow execution via MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionResult {
    /// Workflow execution status.
    pub status: WorkflowStatus,
    /// Output data from the workflow.
    pub output: Option<Value>,
    /// Error information if execution failed.
    pub error: Option<String>,
    /// Execution metadata.
    pub metadata: HashMap<String, Value>,
}

/// Status of workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    /// Workflow completed successfully.
    Success,
    /// Workflow failed with an error.
    Failed,
    /// Workflow is still running.
    Running,
    /// Workflow was cancelled.
    Cancelled,
}

/// Configuration for MCP server setup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name.
    pub name: String,
    /// Server version.
    pub version: String,
    /// Maximum number of concurrent requests.
    pub max_concurrent_requests: usize,
    /// Request timeout in seconds.
    pub request_timeout_seconds: u64,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "pocketflow-mcp".to_string(),
            version: "0.1.0".to_string(),
            max_concurrent_requests: 10,
            request_timeout_seconds: 30,
        }
    }
}

/// Simple workflow node that can be exposed as an MCP tool.
#[derive(Debug, Clone)]
pub struct McpToolNode {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// Input schema for the tool.
    pub input_schema: Value,
}

impl McpToolNode {
    /// Create a new MCP tool node.
    pub fn new(name: String, description: String, input_schema: Value) -> Self {
        Self {
            name,
            description,
            input_schema,
        }
    }

    /// Execute the tool with given input by calling a Node.
    pub async fn execute_with_node<T: Node + ?Sized>(
        &self,
        input: Value,
        mut context: Context,
        node: &T,
    ) -> Result<Value> {
        // Store input in context for the node to access
        context
            .insert(input)
            .map_err(|e| McpError::ToolExecutionFailed {
                message: format!("Failed to set input in context: {e}"),
            })?;

        let (updated_context, _state) =
            node.execute(context)
                .await
                .map_err(|e| McpError::ToolExecutionFailed {
                    message: format!("Node execution failed: {e}"),
                })?;

        // Try to extract result from context
        Ok(updated_context
            .get::<Value>()
            .cloned()
            .unwrap_or(Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_mcp_handler() {
        let handler = WorkflowMcpHandler::new();

        // Test that handler starts empty
        assert_eq!(handler.list_tools().await.len(), 0);
        assert_eq!(handler.list_resources().await.len(), 0);
    }

    #[test]
    fn test_mcp_server_config_default() {
        let config = McpServerConfig::default();
        assert_eq!(config.name, "pocketflow-mcp");
        assert_eq!(config.version, "0.1.0");
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.request_timeout_seconds, 30);
    }
}
