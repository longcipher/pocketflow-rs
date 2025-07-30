use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::error::{Result, ToolError};

/// Core trait for all tools in the system
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's unique name
    fn name(&self) -> &str;

    /// Get the tool's description
    fn description(&self) -> &str;

    /// Get the tool's category
    fn category(&self) -> ToolCategory;

    /// Get parameter schema for the tool
    fn parameter_schema(&self) -> serde_json::Value;

    /// Execute the tool with given parameters
    async fn execute(&self, parameters: ToolParameters, context: ToolContext)
    -> Result<ToolResult>;

    /// Validate parameters before execution (optional override)
    async fn validate_parameters(&self, parameters: &ToolParameters) -> Result<()> {
        // Default implementation does basic JSON schema validation
        self.validate_against_schema(parameters)
    }

    /// Check if the tool supports streaming
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Get tool capabilities
    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Basic]
    }

    /// Get estimated execution time
    fn estimated_duration(&self) -> Option<std::time::Duration> {
        None
    }

    /// Check if the tool is available (e.g., external service is reachable)
    async fn is_available(&self) -> bool {
        true
    }

    /// Get tool metadata
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }

    // Helper method for schema validation
    fn validate_against_schema(&self, parameters: &ToolParameters) -> Result<()> {
        let schema = self.parameter_schema();
        let instance = serde_json::to_value(parameters.inner())?;

        // Use jsonschema crate for validation
        let compiled = jsonschema::Validator::new(&schema)
            .map_err(|e| ToolError::validation(format!("Invalid schema: {e}")))?;

        match compiled.validate(&instance) {
            Ok(()) => Ok(()),
            Err(error) => Err(ToolError::validation(format!(
                "Parameter validation failed: {error}"
            ))),
        }
    }
}

/// Tool category for organization and discovery
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    /// Web-related tools (HTTP, scraping, APIs)
    Web,
    /// File system operations
    File,
    /// System commands and processes
    System,
    /// Database operations
    Database,
    /// AI and ML tools
    AI,
    /// Communication tools (email, slack, etc.)
    Communication,
    /// Development tools (git, build, etc.)
    Development,
    /// Data processing and analysis
    DataProcessing,
    /// Monitoring and observability
    Monitoring,
    /// Security and authentication
    Security,
    /// MCP tools from external servers
    MCP,
    /// Custom user-defined tools
    Custom,
}

/// Tool capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCapability {
    /// Basic synchronous execution
    Basic,
    /// Supports streaming responses
    Streaming,
    /// Supports batch operations
    Batch,
    /// Requires authentication
    Authenticated,
    /// Can be cached
    Cacheable,
    /// Idempotent operation
    Idempotent,
    /// Potentially long-running
    LongRunning,
    /// Requires external network access
    NetworkRequired,
    /// Modifies external state
    StateMutating,
    /// Read-only operation
    ReadOnly,
}

/// Tool parameters wrapper with validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    inner: serde_json::Value,
}

impl ToolParameters {
    pub fn new(value: serde_json::Value) -> Self {
        Self { inner: value }
    }

    pub fn empty() -> Self {
        Self {
            inner: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn new_schema() -> Self {
        Self {
            inner: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    pub fn inner(&self) -> &serde_json::Value {
        &self.inner
    }

    pub fn get<T>(&self, key: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self
            .inner
            .get(key)
            .ok_or_else(|| ToolError::invalid_field(key, "Parameter not found"))?;
        serde_json::from_value(value.clone())
            .map_err(|_| ToolError::invalid_field(key, "Invalid parameter type"))
    }

    pub fn get_optional<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.inner.get(key) {
            Some(value) => {
                if value.is_null() {
                    Ok(None)
                } else {
                    Ok(Some(serde_json::from_value(value.clone()).map_err(
                        |_| ToolError::invalid_field(key, "Invalid parameter type"),
                    )?))
                }
            }
            None => Ok(None),
        }
    }

    pub fn get_string(&self, key: &str) -> Result<String> {
        self.get(key)
    }

    pub fn get_string_optional(&self, key: &str) -> Result<Option<String>> {
        self.get_optional(key)
    }

    pub fn get_bool(&self, key: &str) -> Result<bool> {
        self.get(key)
    }

    pub fn get_bool_optional(&self, key: &str) -> Result<Option<bool>> {
        self.get_optional(key)
    }

    pub fn get_number<T>(&self, key: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.get(key)
    }

    pub fn get_number_optional<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.get_optional(key)
    }

    pub fn get_array<T>(&self, key: &str) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.get(key)
    }

    pub fn get_object(&self, key: &str) -> Result<serde_json::Map<String, serde_json::Value>> {
        self.get(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }

    pub fn keys(&self) -> Vec<String> {
        match &self.inner {
            serde_json::Value::Object(map) => map.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }

    // Schema building methods
    pub fn add_required(mut self, name: &str, param_type: &str, description: &str) -> Self {
        let mut obj = match self.inner {
            serde_json::Value::Object(o) => o,
            _ => serde_json::Map::new(),
        };

        // Add to properties
        let mut properties = obj
            .get("properties")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        properties.insert(
            name.to_string(),
            json!({
                "type": param_type,
                "description": description
            }),
        );

        obj.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );

        // Add to required array
        let mut required = obj
            .get("required")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if !required.contains(&json!(name)) {
            required.push(json!(name));
        }

        obj.insert("required".to_string(), serde_json::Value::Array(required));
        obj.insert("type".to_string(), json!("object"));

        self.inner = serde_json::Value::Object(obj);
        self
    }

    pub fn add_optional(
        mut self,
        name: &str,
        param_type: &str,
        description: &str,
        default_value: Option<serde_json::Value>,
    ) -> Self {
        let mut obj = match self.inner {
            serde_json::Value::Object(o) => o,
            _ => serde_json::Map::new(),
        };

        // Add to properties
        let mut properties = obj
            .get("properties")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let mut param_schema = json!({
            "type": param_type,
            "description": description
        });

        if let Some(default) = default_value
            && let Some(obj) = param_schema.as_object_mut()
        {
            obj.insert("default".to_string(), default);
        }

        properties.insert(name.to_string(), param_schema);
        obj.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        obj.insert("type".to_string(), json!("object"));

        self.inner = serde_json::Value::Object(obj);
        self
    }
}

impl From<serde_json::Value> for ToolParameters {
    fn from(value: serde_json::Value) -> Self {
        Self::new(value)
    }
}

impl From<ToolParameters> for serde_json::Value {
    fn from(params: ToolParameters) -> Self {
        params.inner
    }
}

impl std::fmt::Display for ToolParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: Uuid,
    pub user_id: Option<String>,
    pub workspace_path: Option<std::path::PathBuf>,
    pub environment_variables: HashMap<String, String>,
    pub timeout: Option<std::time::Duration>,
    pub retry_config: RetryConfig,
    pub cache_config: CacheConfig,
    pub custom: HashMap<String, serde_json::Value>,
}

impl ToolContext {
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            environment_variables: HashMap::new(),
            timeout: Some(std::time::Duration::from_secs(30)),
            retry_config: RetryConfig::default(),
            cache_config: CacheConfig::default(),
            custom: HashMap::new(),
        }
    }

    pub fn with_session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = session_id;
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_workspace(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.workspace_path = Some(path.into());
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment_variables.insert(key.into(), value.into());
        self
    }

    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    pub fn get_custom(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom.get(key)
    }

    pub fn get_env_var(&self, key: &str) -> Option<&String> {
        self.environment_variables.get(key)
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Retry configuration for tool execution
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay: std::time::Duration,
    pub max_delay: std::time::Duration,
    pub backoff_multiplier: f64,
    pub retryable_errors: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(10),
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                "network".to_string(),
                "timeout".to_string(),
                "execution".to_string(),
            ],
        }
    }
}

/// Cache configuration for tool results
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl: Option<std::time::Duration>,
    pub max_size: Option<usize>,
    pub cache_key_fields: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl: Some(std::time::Duration::from_secs(300)), // 5 minutes
            max_size: Some(1000),
            cache_key_fields: vec!["*".to_string()], // Cache all parameters
        }
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub content_type: ContentType,
    pub metadata: HashMap<String, serde_json::Value>,
    pub execution_time: Option<std::time::Duration>,
    pub cached: bool,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            content_type: ContentType::Text,
            metadata: HashMap::new(),
            execution_time: None,
            cached: false,
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            content: String::new(),
            content_type: ContentType::Text,
            metadata: HashMap::new(),
            execution_time: None,
            cached: false,
            error: Some(message.into()),
        }
    }

    pub fn with_content_type(mut self, content_type: ContentType) -> Self {
        self.content_type = content_type;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn with_execution_time(mut self, duration: std::time::Duration) -> Self {
        self.execution_time = Some(duration);
        self
    }

    pub fn with_cached(mut self, cached: bool) -> Self {
        self.cached = cached;
        self
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn is_error(&self) -> bool {
        !self.success
    }

    pub fn get_metadata<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.metadata
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Content types for tool results
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Json,
    Html,
    Xml,
    Binary,
    Image,
    Audio,
    Video,
    Archive,
    Document,
    Custom(String),
}

impl ContentType {
    pub fn from_mime_type(mime_type: &str) -> Self {
        match mime_type {
            "text/plain" => Self::Text,
            "application/json" => Self::Json,
            "text/html" => Self::Html,
            "application/xml" | "text/xml" => Self::Xml,
            mime if mime.starts_with("image/") => Self::Image,
            mime if mime.starts_with("audio/") => Self::Audio,
            mime if mime.starts_with("video/") => Self::Video,
            _ => Self::Custom(mime_type.to_string()),
        }
    }

    pub fn to_mime_type(&self) -> &str {
        match self {
            Self::Text => "text/plain",
            Self::Json => "application/json",
            Self::Html => "text/html",
            Self::Xml => "application/xml",
            Self::Binary => "application/octet-stream",
            Self::Image => "image/*",
            Self::Audio => "audio/*",
            Self::Video => "video/*",
            Self::Archive => "application/zip",
            Self::Document => "application/pdf",
            Self::Custom(mime) => mime,
        }
    }
}

/// Tool information for registration and discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub capabilities: Vec<ToolCapability>,
    pub parameter_schema: serde_json::Value,
    pub metadata: ToolMetadata,
}

impl ToolInfo {
    pub fn new<T: Tool>(tool: &T) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            category: tool.category(),
            capabilities: tool.capabilities(),
            parameter_schema: tool.parameter_schema(),
            metadata: tool.metadata(),
        }
    }
}

/// Tool metadata for additional information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub version: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub documentation_url: Option<String>,
    pub source_url: Option<String>,
    pub tags: Vec<String>,
    pub estimated_cost: Option<f64>,
    pub rate_limit: Option<RateLimit>,
    pub custom: HashMap<String, serde_json::Value>,
}

impl ToolMetadata {
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_rate_limit(mut self, rate_limit: RateLimit) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: Option<f64>,
    pub requests_per_minute: Option<u32>,
    pub requests_per_hour: Option<u32>,
    pub burst_size: Option<u32>,
}

impl RateLimit {
    pub fn per_second(requests: f64) -> Self {
        Self {
            requests_per_second: Some(requests),
            requests_per_minute: None,
            requests_per_hour: None,
            burst_size: None,
        }
    }

    pub fn per_minute(requests: u32) -> Self {
        Self {
            requests_per_second: None,
            requests_per_minute: Some(requests),
            requests_per_hour: None,
            burst_size: None,
        }
    }

    pub fn per_hour(requests: u32) -> Self {
        Self {
            requests_per_second: None,
            requests_per_minute: None,
            requests_per_hour: Some(requests),
            burst_size: None,
        }
    }

    pub fn with_burst(mut self, burst_size: u32) -> Self {
        self.burst_size = Some(burst_size);
        self
    }
}
