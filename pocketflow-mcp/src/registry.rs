//! MCP client and server registry for managing multiple connections.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use super::{client::McpClient, error::Result, server::WorkflowMcpHandler};

/// Registry for managing MCP clients and servers.
#[derive(Default)]
pub struct McpRegistry {
    /// Registered MCP clients.
    clients: RwLock<HashMap<String, Arc<dyn McpClient>>>,
    /// Registered MCP servers.
    servers: RwLock<HashMap<String, WorkflowMcpHandler>>,
}

impl McpRegistry {
    /// Create a new MCP registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an MCP client.
    pub async fn register_client(&self, name: String, client: Arc<dyn McpClient>) -> Result<()> {
        self.clients.write().await.insert(name, client);
        Ok(())
    }

    /// Get a registered MCP client.
    pub async fn get_client(&self, name: &str) -> Option<Arc<dyn McpClient>> {
        self.clients.read().await.get(name).cloned()
    }

    /// List all registered client names.
    pub async fn list_clients(&self) -> Vec<String> {
        self.clients.read().await.keys().cloned().collect()
    }

    /// Remove a registered client.
    pub async fn remove_client(&self, name: &str) -> Result<()> {
        self.clients.write().await.remove(name);
        Ok(())
    }

    /// Register an MCP server.
    pub async fn register_server(&self, name: String, server: WorkflowMcpHandler) -> Result<()> {
        self.servers.write().await.insert(name, server);
        Ok(())
    }

    /// Get a registered MCP server.
    pub async fn get_server(&self, name: &str) -> Option<WorkflowMcpHandler> {
        self.servers.read().await.get(name).cloned()
    }

    /// List all registered server names.
    pub async fn list_servers(&self) -> Vec<String> {
        self.servers.read().await.keys().cloned().collect()
    }

    /// Remove a registered server.
    pub async fn remove_server(&self, name: &str) -> Result<()> {
        self.servers.write().await.remove(name);
        Ok(())
    }

    /// Get all registered client and server names.
    pub async fn list_all(&self) -> (Vec<String>, Vec<String>) {
        let clients = self.list_clients().await;
        let servers = self.list_servers().await;
        (clients, servers)
    }

    /// Clear all registered clients and servers.
    pub async fn clear_all(&self) -> Result<()> {
        self.clients.write().await.clear();
        self.servers.write().await.clear();
        Ok(())
    }

    /// Check if a client is registered.
    pub async fn has_client(&self, name: &str) -> bool {
        self.clients.read().await.contains_key(name)
    }

    /// Check if a server is registered.
    pub async fn has_server(&self, name: &str) -> bool {
        self.servers.read().await.contains_key(name)
    }

    /// Get the count of registered clients.
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Get the count of registered servers.
    pub async fn server_count(&self) -> usize {
        self.servers.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = McpRegistry::new();
        assert_eq!(registry.client_count().await, 0);
        assert_eq!(registry.server_count().await, 0);
    }

    #[tokio::test]
    async fn test_registry_operations() {
        let registry = McpRegistry::new();

        // Test initial state
        let (clients, servers) = registry.list_all().await;
        assert!(clients.is_empty());
        assert!(servers.is_empty());

        // Test client operations
        assert!(!registry.has_client("test_client").await);
        assert!(!registry.has_server("test_server").await);

        // Test counts
        assert_eq!(registry.client_count().await, 0);
        assert_eq!(registry.server_count().await, 0);
    }
}
