use std::{collections::HashMap, sync::Arc, time::Duration};

use serde_json::{Value, json};
use tokio::sync::RwLock;

use crate::{
    core::{Tool, ToolCapability, ToolCategory, ToolContext, ToolParameters, ToolResult},
    error::{ErrorCategory, Result, ToolError},
};

/// Tool registry for managing and executing tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
    categories: Arc<RwLock<HashMap<String, String>>>,
    category_tools: Arc<RwLock<HashMap<String, Vec<String>>>>,
    cache_enabled: bool,
    cache: Arc<RwLock<HashMap<String, (ToolResult, std::time::Instant)>>>,
    cache_ttl: Duration,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            category_tools: Arc::new(RwLock::new(HashMap::new())),
            cache_enabled: false,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Enable caching with TTL
    pub fn with_cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Set cache TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Register a new tool category
    pub async fn register_category(&mut self, name: &str, description: &str) -> Result<()> {
        {
            let mut categories = self.categories.write().await;
            categories.insert(name.to_string(), description.to_string());
        }
        {
            let mut category_tools = self.category_tools.write().await;
            category_tools.insert(name.to_string(), Vec::new());
        }
        Ok(())
    }

    /// Register a tool in the registry
    pub async fn register_tool(&mut self, tool: Box<dyn Tool>) -> Result<()> {
        let tool_name = tool.name().to_string();

        // Add to tools collection
        {
            let mut tools = self.tools.write().await;
            tools.insert(tool_name.clone(), tool);
        }

        // Add to "general" category if no specific category found
        {
            let mut category_tools = self.category_tools.write().await;
            if let Some(general_tools) = category_tools.get_mut("general") {
                general_tools.push(tool_name);
            } else {
                // Create general category if it doesn't exist
                category_tools.insert("general".to_string(), vec![tool_name]);
            }
        }

        Ok(())
    }

    /// Execute a tool by name
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        // Get tool
        let tools = self.tools.read().await;
        let tool = tools
            .get(tool_name)
            .ok_or_else(|| ToolError::not_found(tool_name))?;

        // Check cache first
        if tool.capabilities().contains(&ToolCapability::Cacheable) {
            let cache_key = format!("{tool_name}:{parameters}");
            let cache = self.cache.read().await;
            if let Some((cached_result, _timestamp)) = cache.get(&cache_key) {
                return Ok(cached_result.clone().with_cached(true));
            }
        }

        // Validate parameters (convert Value to ToolParameters)
        let tool_params = ToolParameters::new(parameters.clone());
        if let Err(e) = tool.validate_parameters(&tool_params).await {
            return Err(ToolError::invalid_parameters(e.to_string()));
        }

        // Execute tool
        let result = tool.execute(tool_params, context.clone()).await?;

        // Cache result if caching is enabled
        if self.cache_enabled {
            let cache_key = format!("{tool_name}:{parameters}");
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, (result.clone(), std::time::Instant::now()));
        }

        Ok(result)
    }

    /// Execute multiple tools in batch
    pub async fn execute_batch(&self, requests: Vec<ToolRequest>) -> Vec<Result<ToolResult>> {
        let mut results = Vec::new();

        // Execute tools in parallel
        let handles: Vec<_> = requests
            .into_iter()
            .map(|req| {
                let registry = self.clone();
                tokio::spawn(async move {
                    registry
                        .execute_tool(&req.tool_name, &req.parameters, &req.context)
                        .await
                })
            })
            .collect();

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(ToolError::execution(e.to_string()))),
            }
        }

        results
    }

    /// Get a tool by name
    pub async fn get_tool(&self, name: &str) -> Option<Box<dyn Tool>> {
        let tools = self.tools.read().await;
        // Note: This is a simplified implementation
        // In practice, you'd need to handle trait objects differently
        tools.get(name).map(|_| {
            // Return a placeholder - real implementation would clone the tool
            Box::new(MockTool::new(name)) as Box<dyn Tool>
        })
    }

    /// List all tool names
    pub async fn list_tools(&self) -> Vec<String> {
        let tools = self.tools.read().await;
        tools.keys().cloned().collect()
    }

    /// List all categories
    pub async fn list_categories(&self) -> Vec<String> {
        let categories = self.categories.read().await;
        categories.keys().cloned().collect()
    }

    /// List tools in a specific category
    pub async fn list_tools_in_category(&self, category: &str) -> Vec<String> {
        let category_tools = self.category_tools.read().await;
        category_tools.get(category).cloned().unwrap_or_default()
    }

    /// Find tools by capability
    pub async fn find_tools_by_capability(&self, capability: ToolCapability) -> Vec<String> {
        let tools = self.tools.read().await;
        let mut matching_tools = Vec::new();

        for (name, tool) in tools.iter() {
            if tool.capabilities().contains(&capability) {
                matching_tools.push(name.clone());
            }
        }

        matching_tools
    }

    /// Find tools by name pattern
    pub async fn find_tools_by_name_pattern(&self, pattern: &str) -> Vec<String> {
        let tools = self.tools.read().await;
        tools
            .keys()
            .filter(|name| name.contains(pattern))
            .cloned()
            .collect()
    }

    /// Find tools requiring specific permission
    pub async fn find_tools_requiring_permission(&self, _permission: ErrorCategory) -> Vec<String> {
        let tools = self.tools.read().await;
        let matching_tools = Vec::new();

        for (_name, _tool) in tools.iter() {
            // TODO: Implement required_permissions() method on Tool trait
            // if tool.required_permissions().contains(&permission) {
            //     matching_tools.push(name.clone());
            // }
        }

        matching_tools
    }

    /// Validate a tool call before execution
    pub async fn validate_tool_call(&self, tool_name: &str, parameters: &Value) -> Result<()> {
        let tools = self.tools.read().await;
        let tool = tools
            .get(tool_name)
            .ok_or_else(|| ToolError::not_found(tool_name))?;

        let tool_params = ToolParameters::new(parameters.clone());
        tool.validate_parameters(&tool_params).await?;
        Ok(())
    }

    /// Check if a tool has a specific capability
    pub async fn tool_has_capability(&self, tool_name: &str, capability: ToolCapability) -> bool {
        let tools = self.tools.read().await;
        if let Some(tool) = tools.get(tool_name) {
            tool.capabilities().contains(&capability)
        } else {
            false
        }
    }

    /// Execute tool with retry logic
    pub async fn execute_tool_with_retry(
        &self,
        tool_name: &str,
        parameters: &Value,
        context: &ToolContext,
        retry_config: RetryConfig,
    ) -> Result<ToolResult> {
        let mut attempts = 0;
        let mut delay = retry_config.initial_delay;

        loop {
            attempts += 1;

            match self.execute_tool(tool_name, parameters, context).await {
                Ok(result) => return Ok(result),
                Err(e) if attempts >= retry_config.max_attempts => return Err(e),
                Err(_) => {
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, retry_config.max_delay);
                }
            }
        }
    }

    /// Execute tool with fallback to alternatives
    pub async fn execute_tool_with_fallback(
        &self,
        tool_names: &[&str],
        parameters: &Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let mut last_error = None;

        for &tool_name in tool_names {
            match self.execute_tool(tool_name, parameters, context).await {
                Ok(result) => return Ok(result),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| ToolError::not_found("No tools provided")))
    }

    /// Register MCP server tools
    pub async fn register_mcp_server(
        &mut self,
        server_url: &str,
        tool_names: Vec<&str>,
    ) -> Result<()> {
        // This would integrate with pocketflow-mcp for real MCP connections
        for tool_name in tool_names {
            let mcp_tool = McpToolWrapper::new(tool_name, server_url);
            self.register_tool(Box::new(mcp_tool)).await?;
        }
        Ok(())
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let now = std::time::Instant::now();
        let expired_count = cache
            .values()
            .filter(|(_, timestamp)| now.duration_since(*timestamp) > self.cache_ttl)
            .count();

        CacheStats {
            total_entries: cache.len(),
            expired_entries: expired_count,
            hit_rate: 0.0, // Would track this in real implementation
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool request for batch execution
#[derive(Debug, Clone)]
pub struct ToolRequest {
    pub tool_name: String,
    pub parameters: Value,
    pub context: ToolContext,
}

/// Retry configuration for tool execution
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
        }
    }
}

impl RetryConfig {
    pub fn with_max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub hit_rate: f64,
}

/// Tool composition for chaining tools together
#[derive(Debug, Clone)]
pub struct ToolComposition {
    pub name: String,
    pub steps: Vec<CompositionStep>,
}

#[derive(Debug, Clone)]
pub struct CompositionStep {
    pub name: String,
    pub tool_name: String,
    pub parameters: Value,
}

impl ToolComposition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            steps: Vec::new(),
        }
    }

    pub fn add_step(mut self, step_name: &str, tool_name: &str, parameters: Value) -> Self {
        self.steps.push(CompositionStep {
            name: step_name.to_string(),
            tool_name: tool_name.to_string(),
            parameters,
        });
        self
    }

    pub async fn execute(&self, registry: &ToolRegistry, context: &ToolContext) -> Result<Value> {
        let mut results = json!({});
        let execution_context = context.clone();

        for step in &self.steps {
            // Replace template variables in parameters
            let resolved_params = self.resolve_template_variables(&step.parameters, &results)?;

            // Execute step
            let result = registry
                .execute_tool(&step.tool_name, &resolved_params, &execution_context)
                .await?;

            // Store result for next steps (use content instead of data)
            results[&step.name] = result.content.clone().into();

            // TODO: Update context with step result - ToolContext doesn't have with_variable method
            // execution_context = execution_context.with_variable(&step.name, result.content.clone());
        }

        Ok(results)
    }

    fn resolve_template_variables(&self, params: &Value, results: &Value) -> Result<Value> {
        // Simplified template resolution - in practice would be more sophisticated
        let params_str = params.to_string();

        // Replace {{ step.field }} patterns with actual values
        let mut resolved = params_str.clone();

        // This is a basic implementation - real version would use proper templating
        if let Ok(regex) = regex::Regex::new(r"\{\{\s*(\w+)\.(\w+)\s*\}}")
            && let Some(captures) = regex.captures(&params_str)
            && let (Some(step), Some(field)) = (captures.get(1), captures.get(2))
            && let Some(step_result) = results.get(step.as_str())
            && let Some(field_value) = step_result.get(field.as_str())
        {
            resolved = resolved.replace(&captures[0], &field_value.to_string());
        }

        serde_json::from_str(&resolved)
            .map_err(|e| ToolError::invalid_parameters(format!("Template resolution failed: {e}")))
    }
}

// Mock implementations for compilation

use async_trait::async_trait;

struct MockTool {
    name: String,
}

impl MockTool {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        "Mock tool for testing"
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Custom
    }
    fn parameter_schema(&self) -> serde_json::Value {
        serde_json::json!({"description": "Mock tool parameters"})
    }
    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![]
    }

    async fn execute(
        &self,
        _parameters: ToolParameters,
        _context: ToolContext,
    ) -> Result<ToolResult> {
        Ok(ToolResult::success(
            serde_json::json!({"mock": true}).to_string(),
        ))
    }
}

struct McpToolWrapper {
    name: String,
    server_url: String,
}

impl McpToolWrapper {
    fn new(name: &str, server_url: &str) -> Self {
        Self {
            name: name.to_string(),
            server_url: server_url.to_string(),
        }
    }
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        "MCP tool wrapper"
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::MCP
    }
    fn parameter_schema(&self) -> serde_json::Value {
        serde_json::json!({"description": "MCP tool parameters"})
    }
    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::NetworkRequired]
    }

    async fn execute(
        &self,
        parameters: ToolParameters,
        _context: ToolContext,
    ) -> Result<ToolResult> {
        // Would call actual MCP server here
        Ok(ToolResult::success(
            serde_json::json!({
                "mcp_server": self.server_url,
                "tool": self.name,
                "params": parameters.to_string()
            })
            .to_string(),
        ))
    }
}
