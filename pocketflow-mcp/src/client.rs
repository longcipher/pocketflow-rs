//! MCP client functionality for calling external MCP tools.

use std::collections::HashMap;

use async_trait::async_trait;
use pocketflow_core::{
    context::Context, error::Result as FlowResult, node::Node, state::FlowState,
};
use serde_json::Value;
use ultrafast_mcp::{UltraFastClient, types::ResourceContent};

use super::{
    ClientCapabilities, ClientInfo, ListResourcesRequest, ListToolsRequest, ReadResourceRequest,
    Resource, Result, ServerInfo, Tool, ToolCall, ToolContent,
    error::McpError as PocketFlowMcpError,
};

/// Configuration for MCP transport connections.
#[derive(Debug, Clone)]
pub enum McpTransportConfig {
    /// Connect to an MCP server via child process (stdio)
    Stdio,
    /// Connect to an MCP server via HTTP
    Http {
        /// HTTP endpoint URL
        url: String,
    },
    /// Connect with custom configuration
    Custom {
        /// Custom configuration string
        config: String,
    },
}

/// Trait for MCP client operations.
#[async_trait]
pub trait McpClient: Send + Sync {
    /// List available tools from the MCP server.
    async fn list_tools(&self) -> Result<Vec<Tool>>;

    /// Call a specific tool with arguments.
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value>;

    /// List available resources from the MCP server.
    async fn list_resources(&self) -> Result<Vec<Resource>>;

    /// Read a resource by URI.
    async fn read_resource(&self, uri: &str) -> Result<Value>;

    /// Get server information.
    async fn get_server_info(&self) -> Result<ServerInfo>;
}

/// Concrete implementation of MCP client using ultrafast-mcp.
#[derive(Debug)]
pub struct UltraFastMcpClient {
    client: UltraFastClient,
    connected: bool,
}

impl UltraFastMcpClient {
    /// Create a new MCP client with the given transport configuration.
    pub async fn new(
        client_info: ClientInfo,
        capabilities: ClientCapabilities,
        config: McpTransportConfig,
    ) -> Result<Self> {
        let client = UltraFastClient::new(client_info, capabilities);

        let mut mcp_client = Self {
            client,
            connected: false,
        };

        // Connect based on configuration
        match config {
            McpTransportConfig::Stdio => {
                mcp_client.client.connect_stdio().await.map_err(|e| {
                    PocketFlowMcpError::ConnectionFailed {
                        message: e.to_string(),
                    }
                })?;
            }
            McpTransportConfig::Http { url } => {
                mcp_client
                    .client
                    .connect_streamable_http(&url)
                    .await
                    .map_err(|e| PocketFlowMcpError::ConnectionFailed {
                        message: e.to_string(),
                    })?;
            }
            McpTransportConfig::Custom { config: _ } => {
                return Err(PocketFlowMcpError::InvalidArguments {
                    message: "Custom configuration not yet implemented".to_string(),
                });
            }
        }

        mcp_client.connected = true;
        Ok(mcp_client)
    }

    /// Check if the client is connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl McpClient for UltraFastMcpClient {
    async fn list_tools(&self) -> Result<Vec<Tool>> {
        if !self.connected {
            return Err(PocketFlowMcpError::ConnectionFailed {
                message: "Client not connected".to_string(),
            });
        }

        let request = ListToolsRequest { cursor: None };
        let response = self.client.list_tools(request).await.map_err(|e| {
            PocketFlowMcpError::ToolExecutionFailed {
                message: e.to_string(),
            }
        })?;

        Ok(response.tools)
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        if !self.connected {
            return Err(PocketFlowMcpError::ConnectionFailed {
                message: "Client not connected".to_string(),
            });
        }

        let tool_call = ToolCall {
            name: name.to_string(),
            arguments: Some(arguments),
        };

        let result = self.client.call_tool(tool_call).await.map_err(|e| {
            PocketFlowMcpError::ToolExecutionFailed {
                message: e.to_string(),
            }
        })?;

        // Extract text content from the result
        let mut combined_text = String::new();
        for content in result.content {
            match content {
                ToolContent::Text { text } => {
                    combined_text.push_str(&text);
                }
                _ => continue,
            }
        }

        // Try to parse as JSON, fallback to plain text
        serde_json::from_str(&combined_text)
            .or(Ok(Value::String(combined_text)))
            .map_err(PocketFlowMcpError::Serialization)
    }

    async fn list_resources(&self) -> Result<Vec<Resource>> {
        if !self.connected {
            return Err(PocketFlowMcpError::ConnectionFailed {
                message: "Client not connected".to_string(),
            });
        }

        let request = ListResourcesRequest { cursor: None };
        let response = self.client.list_resources(request).await.map_err(|e| {
            PocketFlowMcpError::ToolExecutionFailed {
                message: e.to_string(),
            }
        })?;

        Ok(response.resources)
    }

    async fn read_resource(&self, uri: &str) -> Result<Value> {
        if !self.connected {
            return Err(PocketFlowMcpError::ConnectionFailed {
                message: "Client not connected".to_string(),
            });
        }

        let request = ReadResourceRequest {
            uri: uri.to_string(),
        };

        let response = self.client.read_resource(request).await.map_err(|_e| {
            PocketFlowMcpError::ResourceNotFound {
                uri: uri.to_string(),
            }
        })?;

        // Convert the first resource content to JSON
        if let Some(content) = response.contents.first() {
            match content {
                ResourceContent::Text { text, .. } => serde_json::from_str(text)
                    .or_else(|_| Ok(Value::String(text.to_string())))
                    .map_err(PocketFlowMcpError::Serialization),
                _ => Ok(Value::Null),
            }
        } else {
            Ok(Value::Null)
        }
    }

    async fn get_server_info(&self) -> Result<ServerInfo> {
        // For ultrafast-mcp, server info is typically obtained during connection
        // This is a placeholder implementation
        Ok(ServerInfo {
            name: "Unknown Server".to_string(),
            version: "Unknown".to_string(),
            description: None,
            authors: None,
            homepage: None,
            license: None,
            repository: None,
        })
    }
}

/// A workflow node that acts as an MCP client to call external tools.
#[derive(Debug)]
pub struct McpClientNode<S: FlowState> {
    name: String,
    transport_config: McpTransportConfig,
    tool_name: String,
    input_mapping: HashMap<String, String>,
    output_key: Option<String>,
    on_success: Option<S>,
    on_error: Option<S>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: FlowState> McpClientNode<S> {
    /// Create a new builder for McpClientNode.
    pub fn builder(name: impl Into<String>) -> McpClientNodeBuilder<S> {
        McpClientNodeBuilder::new(name)
    }
}

#[async_trait]
impl<S: FlowState> Node for McpClientNode<S> {
    type State = S;

    async fn execute(&self, mut context: Context) -> FlowResult<(Context, Self::State)> {
        // Create client info and capabilities
        let client_info = ClientInfo {
            name: format!("{}_client", self.name),
            version: "1.0.0".to_string(),
            description: Some("PocketFlow MCP Client".to_string()),
            authors: None,
            homepage: None,
            license: None,
            repository: None,
        };

        let capabilities = ClientCapabilities::default();

        // Create MCP client
        let client =
            UltraFastMcpClient::new(client_info, capabilities, self.transport_config.clone())
                .await
                .map_err(|e| {
                    pocketflow_core::error::FlowError::context(format!(
                        "Failed to create MCP client: {e}"
                    ))
                })?;

        // Prepare tool arguments from context
        let mut tool_args = serde_json::Map::new();
        for (context_key, tool_arg) in &self.input_mapping {
            if let Some(value) = context.get_raw(context_key) {
                tool_args.insert(tool_arg.clone(), value.clone());
            }
        }

        // Call the tool
        let result = client
            .call_tool(&self.tool_name, Value::Object(tool_args))
            .await;

        match result {
            Ok(tool_result) => {
                // Store result in context
                if let Some(output_key) = &self.output_key {
                    context.set(output_key, &tool_result)?;
                }

                // Transition to success state or stay in current state
                let next_state = self.on_success.clone().unwrap_or_else(|| {
                    // Default behavior: extract state from context or use a default
                    // This is a simplified approach - in practice, you might want more sophisticated state handling
                    todo!("Need to determine default success state")
                });

                Ok((context, next_state))
            }
            Err(e) => {
                // Store error in context
                context.set("mcp_error", e.to_string())?;

                // Transition to error state
                let next_state = self
                    .on_error
                    .clone()
                    .unwrap_or_else(|| todo!("Need to determine default error state"));

                Ok((context, next_state))
            }
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Builder for McpClientNode.
pub struct McpClientNodeBuilder<S: FlowState> {
    name: String,
    transport_config: Option<McpTransportConfig>,
    tool_name: Option<String>,
    input_mapping: HashMap<String, String>,
    output_key: Option<String>,
    on_success: Option<S>,
    on_error: Option<S>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: FlowState> McpClientNodeBuilder<S> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport_config: None,
            tool_name: None,
            input_mapping: HashMap::new(),
            output_key: None,
            on_success: None,
            on_error: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Configure the node to use stdio transport.
    pub fn with_stdio(mut self) -> Self {
        self.transport_config = Some(McpTransportConfig::Stdio);
        self
    }

    /// Configure the node to use HTTP transport.
    pub fn with_http(mut self, url: impl Into<String>) -> Self {
        self.transport_config = Some(McpTransportConfig::Http { url: url.into() });
        self
    }

    /// Configure the node to use custom transport.
    pub fn with_custom(mut self, config: impl Into<String>) -> Self {
        self.transport_config = Some(McpTransportConfig::Custom {
            config: config.into(),
        });
        self
    }

    /// Set the tool name to call.
    pub fn tool(mut self, name: impl Into<String>) -> Self {
        self.tool_name = Some(name.into());
        self
    }

    /// Map a context key to a tool argument.
    pub fn map_input(
        mut self,
        context_key: impl Into<String>,
        tool_arg: impl Into<String>,
    ) -> Self {
        self.input_mapping
            .insert(context_key.into(), tool_arg.into());
        self
    }

    /// Set the context key to store the tool result.
    pub fn output_to(mut self, key: impl Into<String>) -> Self {
        self.output_key = Some(key.into());
        self
    }

    /// Set the state to transition to on success.
    pub fn on_success(mut self, state: S) -> Self {
        self.on_success = Some(state);
        self
    }

    /// Set the state to transition to on error.
    pub fn on_error(mut self, state: S) -> Self {
        self.on_error = Some(state);
        self
    }

    /// Build the McpClientNode.
    pub fn build(self) -> Result<McpClientNode<S>> {
        let transport_config =
            self.transport_config
                .ok_or_else(|| PocketFlowMcpError::InvalidArguments {
                    message: "Transport configuration is required".to_string(),
                })?;

        let tool_name = self
            .tool_name
            .ok_or_else(|| PocketFlowMcpError::InvalidArguments {
                message: "Tool name is required".to_string(),
            })?;

        Ok(McpClientNode {
            name: self.name,
            transport_config,
            tool_name,
            input_mapping: self.input_mapping,
            output_key: self.output_key,
            on_success: self.on_success,
            on_error: self.on_error,
            _phantom: std::marker::PhantomData,
        })
    }
}

/// Helper functions for creating common MCP client nodes.
pub mod helpers {
    use super::*;

    /// Create an MCP client node that calls a filesystem tool.
    pub fn filesystem_tool<S: FlowState>(
        name: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> McpClientNodeBuilder<S> {
        McpClientNode::builder(name)
            .with_stdio() // Filesystem servers typically use stdio
            .tool(tool_name)
    }

    /// Create an MCP client node that calls a web search tool.
    pub fn web_search_tool<S: FlowState>(
        name: impl Into<String>,
        tool_name: impl Into<String>,
        server_url: impl Into<String>,
    ) -> McpClientNodeBuilder<S> {
        McpClientNode::builder(name)
            .with_http(server_url)
            .tool(tool_name)
    }

    /// Create an MCP client node that calls a database tool.
    pub fn database_tool<S: FlowState>(
        name: impl Into<String>,
        tool_name: impl Into<String>,
        server_url: impl Into<String>,
    ) -> McpClientNodeBuilder<S> {
        McpClientNode::builder(name)
            .with_http(server_url)
            .tool(tool_name)
    }

    /// Create an MCP client node for general HTTP MCP servers.
    pub fn http_tool<S: FlowState>(
        name: impl Into<String>,
        tool_name: impl Into<String>,
        url: impl Into<String>,
    ) -> McpClientNodeBuilder<S> {
        McpClientNode::builder(name).with_http(url).tool(tool_name)
    }

    /// Create an MCP client node for stdio-based MCP servers.
    pub fn stdio_tool<S: FlowState>(
        name: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> McpClientNodeBuilder<S> {
        McpClientNode::builder(name).with_stdio().tool(tool_name)
    }
}
