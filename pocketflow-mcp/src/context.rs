//! Context extensions for MCP functionality.

use std::{collections::HashMap, sync::Arc};

use pocketflow_core::context::Context;
use serde_json::Value;

use super::{Result, client::McpClient, error::McpError};

/// MCP context extension for workflow contexts.
#[derive(Clone, Default)]
pub struct McpContext {
    /// Registered MCP clients by name.
    pub clients: HashMap<String, Arc<dyn McpClient>>,
    /// MCP tool call results.
    pub tool_results: HashMap<String, Value>,
    /// MCP resource cache.
    pub resource_cache: HashMap<String, Value>,
}

/// Extension trait to add MCP functionality to Context.
#[allow(async_fn_in_trait)]
pub trait McpContextExt {
    /// Register an MCP client with a name.
    fn register_mcp_client(
        &mut self,
        name: impl Into<String>,
        client: Arc<dyn McpClient>,
    ) -> crate::error::Result<()>;

    /// Get a registered MCP client by name.
    fn get_mcp_client(&self, name: &str) -> Option<Arc<dyn McpClient>>;

    /// Remove a registered MCP client.
    fn remove_mcp_client(&mut self, name: &str) -> Option<Arc<dyn McpClient>>;

    /// List all registered MCP client names.
    fn list_mcp_clients(&self) -> Vec<String>;

    /// Call an MCP tool using a registered client.
    async fn call_mcp_tool(
        &self,
        client_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value>;

    /// Store an MCP tool call result.
    fn set_mcp_result(&mut self, key: impl Into<String>, result: Value)
    -> crate::error::Result<()>;

    /// Get an MCP tool call result.
    fn get_mcp_result(&self, key: &str) -> Option<&Value>;

    /// Cache an MCP resource.
    fn cache_mcp_resource(
        &mut self,
        uri: impl Into<String>,
        data: Value,
    ) -> crate::error::Result<()>;

    /// Get a cached MCP resource.
    fn get_cached_resource(&self, uri: &str) -> Option<&Value>;

    /// Clear MCP cache.
    fn clear_mcp_cache(&mut self);
}

impl McpContextExt for Context {
    fn register_mcp_client(
        &mut self,
        name: impl Into<String>,
        client: Arc<dyn McpClient>,
    ) -> crate::error::Result<()> {
        let mut mcp_data = self.get::<McpContext>().cloned().unwrap_or_default();
        mcp_data.clients.insert(name.into(), client);
        self.insert(mcp_data)?;
        Ok(())
    }

    fn get_mcp_client(&self, name: &str) -> Option<Arc<dyn McpClient>> {
        self.get::<McpContext>()?.clients.get(name).cloned()
    }

    fn remove_mcp_client(&mut self, name: &str) -> Option<Arc<dyn McpClient>> {
        let mut mcp_data = self.get::<McpContext>().cloned().unwrap_or_default();
        let result = mcp_data.clients.remove(name);
        let _ = self.insert(mcp_data);
        result
    }

    fn list_mcp_clients(&self) -> Vec<String> {
        self.get::<McpContext>()
            .map(|data| data.clients.keys().cloned().collect())
            .unwrap_or_default()
    }

    async fn call_mcp_tool(
        &self,
        client_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        let client = self
            .get_mcp_client(client_name)
            .ok_or_else(|| McpError::ClientNotFound {
                client_name: client_name.to_string(),
            })?;

        client.call_tool(tool_name, arguments).await
    }

    fn set_mcp_result(
        &mut self,
        key: impl Into<String>,
        result: Value,
    ) -> crate::error::Result<()> {
        let mut mcp_data = self.get::<McpContext>().cloned().unwrap_or_default();
        mcp_data.tool_results.insert(key.into(), result);
        self.insert(mcp_data)?;
        Ok(())
    }

    fn get_mcp_result(&self, key: &str) -> Option<&Value> {
        self.get::<McpContext>()?.tool_results.get(key)
    }

    fn cache_mcp_resource(
        &mut self,
        uri: impl Into<String>,
        data: Value,
    ) -> crate::error::Result<()> {
        let mut mcp_data = self.get::<McpContext>().cloned().unwrap_or_default();
        mcp_data.resource_cache.insert(uri.into(), data);
        self.insert(mcp_data)?;
        Ok(())
    }

    fn get_cached_resource(&self, uri: &str) -> Option<&Value> {
        self.get::<McpContext>()?.resource_cache.get(uri)
    }

    fn clear_mcp_cache(&mut self) {
        let mut mcp_data = self.get::<McpContext>().cloned().unwrap_or_default();
        mcp_data.tool_results.clear();
        mcp_data.resource_cache.clear();
        let _ = self.insert(mcp_data);
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use ultrafast_mcp::{Resource, ServerInfo, Tool};

    use super::*;

    // Mock MCP client for testing
    #[derive(Debug)]
    struct MockMcpClient {
        name: String,
    }

    #[async_trait]
    impl McpClient for MockMcpClient {
        async fn list_tools(&self) -> Result<Vec<Tool>> {
            Ok(vec![])
        }

        async fn call_tool(&self, tool_name: &str, _arguments: Value) -> Result<Value> {
            Ok(serde_json::json!({
                "result": format!("Called {} from {}", tool_name, self.name)
            }))
        }

        async fn list_resources(&self) -> Result<Vec<Resource>> {
            Ok(vec![])
        }

        async fn read_resource(&self, _uri: &str) -> Result<Value> {
            Ok(Value::Null)
        }

        async fn get_server_info(&self) -> Result<ServerInfo> {
            Ok(ServerInfo::new(
                "context_server".to_string(),
                "0.1.0".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn test_mcp_context_extensions() {
        let mut context = Context::new();

        // Register a mock client
        let client = Arc::new(MockMcpClient {
            name: "test_client".to_string(),
        });
        context.register_mcp_client("test", client.clone()).unwrap();

        // Check client registration
        assert!(context.get_mcp_client("test").is_some());
        assert_eq!(context.list_mcp_clients(), vec!["test"]);

        // Test tool calling
        let result = context
            .call_mcp_tool("test", "test_tool", serde_json::json!({}))
            .await
            .unwrap();

        assert!(result.is_object());

        // Test result storage
        context
            .set_mcp_result("test_result", result.clone())
            .unwrap();
        assert_eq!(context.get_mcp_result("test_result"), Some(&result));

        // Test resource caching
        let resource_data = serde_json::json!({"content": "test resource"});
        context
            .cache_mcp_resource("test://resource", resource_data.clone())
            .unwrap();
        assert_eq!(
            context.get_cached_resource("test://resource"),
            Some(&resource_data)
        );

        // Test clearing cache
        context.clear_mcp_cache();
        assert!(context.get_mcp_result("test_result").is_none());
        assert!(context.get_cached_resource("test://resource").is_none());
    }
}
