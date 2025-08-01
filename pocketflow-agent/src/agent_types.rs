//! Agent types and configuration for PocketFlow AI agents.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent capabilities that define what an agent can do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentCapability {
    /// Basic agent functionality
    Basic,
    /// Tool calling capability
    ToolCalling,
    /// Code execution capability
    CodeExecution,
    /// Planning and reasoning
    Planning,
    /// Multi-agent coordination
    Coordination,
    /// Streaming responses
    Streaming,
    /// Memory persistence
    Memory,
    /// Learning from interactions
    Learning,
    /// Error recovery
    ErrorRecovery,
    /// Custom capability
    Custom(String),
}

/// Agent role in multi-agent systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// Independent agent
    Independent,
    /// Coordinator agent
    Coordinator,
    /// Specialist agent
    Specialist { domain: String },
    /// Worker agent
    Worker,
    /// Reviewer agent
    Reviewer,
    /// Custom role
    Custom(String),
}

/// Agent execution mode determining how the agent processes requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Synchronous execution
    Sync,
    /// Asynchronous execution
    Async,
    /// Streaming execution
    Streaming,
    /// Background execution
    Background,
}

/// Agent priority level for task scheduling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Configuration for an AI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent unique identifier
    pub id: Uuid,
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Agent role
    pub role: AgentRole,
    /// Agent capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Execution mode
    pub execution_mode: ExecutionMode,
    /// Priority level
    pub priority: Priority,
    /// Maximum execution steps
    pub max_steps: usize,
    /// Execution timeout
    pub timeout: Option<std::time::Duration>,
    /// Model configuration
    pub model_config: ModelConfig,
    /// System prompt
    pub system_prompt: String,
    /// Tool names accessible to this agent
    pub available_tools: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentConfig {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            role: AgentRole::Independent,
            capabilities: vec![AgentCapability::Basic],
            execution_mode: ExecutionMode::Sync,
            priority: Priority::Normal,
            max_steps: 10,
            timeout: Some(std::time::Duration::from_secs(300)), // 5 minutes
            model_config: ModelConfig::default(),
            system_prompt: "You are a helpful AI assistant.".to_string(),
            available_tools: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_role(mut self, role: AgentRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<AgentCapability>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_model_config(mut self, config: ModelConfig) -> Self {
        self.model_config = config;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.available_tools = tools;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn has_capability(&self, capability: &AgentCapability) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn can_use_tool(&self, tool_name: &str) -> bool {
        self.available_tools.contains(&tool_name.to_string())
    }
}

/// Model configuration for integrating with AI providers via genai.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model provider
    pub provider: ModelProvider,
    /// Model name
    pub model_name: String,
    /// Model parameters
    pub parameters: ModelParameters,
    /// API configuration
    pub api_config: ApiConfig,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: ModelProvider::OpenAI,
            model_name: "gpt-4o-mini".to_string(),
            parameters: ModelParameters::default(),
            api_config: ApiConfig::default(),
        }
    }
}

impl ModelConfig {
    pub fn new(provider: ModelProvider, model_name: impl Into<String>) -> Self {
        Self {
            provider,
            model_name: model_name.into(),
            parameters: ModelParameters::default(),
            api_config: ApiConfig::default(),
        }
    }

    pub fn with_parameters(mut self, parameters: ModelParameters) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_api_config(mut self, config: ApiConfig) -> Self {
        self.api_config = config;
        self
    }
}

/// Supported AI model providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelProvider {
    /// OpenAI provider (GPT models)
    OpenAI,
    /// Anthropic provider (Claude models)
    Anthropic,
    /// Google provider (Gemini models)
    Google,
    /// Ollama local models
    Ollama,
    /// Custom provider with name and base URL
    Custom { name: String, base_url: String },
}

/// Parameters for model inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Controls randomness (0.0 = deterministic, 1.0 = very random)
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: Option<usize>,
    /// Nucleus sampling parameter
    pub top_p: Option<f32>,
    /// Frequency penalty for repetition reduction
    pub frequency_penalty: Option<f32>,
    /// Presence penalty for topic diversity
    pub presence_penalty: Option<f32>,
    /// Stop sequences to end generation
    pub stop_sequences: Vec<String>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: Some(4096),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: Vec::new(),
        }
    }
}

impl ModelParameters {
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = stop_sequences;
        self
    }
}

/// API configuration for connecting to AI services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Base URL for the API endpoint
    pub base_url: Option<String>,
    /// Request timeout duration
    pub timeout: std::time::Duration,
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Delay between retry attempts
    pub retry_delay: std::time::Duration,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            timeout: std::time::Duration::from_secs(60),
            max_retries: 3,
            retry_delay: std::time::Duration::from_millis(1000),
        }
    }
}

impl ApiConfig {
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// Information about a single execution step performed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    /// Sequential step number
    pub step_number: usize,
    /// When the step was executed
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Type of step performed
    pub step_type: AgentStepType,
    /// Input data for the step
    pub input: serde_json::Value,
    /// Output data from the step (if any)
    pub output: Option<serde_json::Value>,
    /// Error message (if step failed)
    pub error: Option<String>,
    /// How long the step took to execute
    pub duration: Option<std::time::Duration>,
    /// Additional metadata about the step
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentStep {
    pub fn new(step_number: usize, step_type: AgentStepType, input: serde_json::Value) -> Self {
        Self {
            step_number,
            timestamp: chrono::Utc::now(),
            step_type,
            input,
            output: None,
            error: None,
            duration: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn with_duration(mut self, duration: std::time::Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn is_successful(&self) -> bool {
        self.error.is_none()
    }
}

/// Types of steps an agent can perform during execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStepType {
    /// Thinking/reasoning step
    Thinking,
    /// Tool call step
    ToolCall { tool_name: String },
    /// Agent delegation step
    Delegation { target_agent: String },
    /// Model inference step
    ModelInference,
    /// Validation step
    Validation,
    /// Planning step
    Planning,
    /// Memory update step
    MemoryUpdate,
    /// Response step
    Response,
    /// Custom step type
    Custom(String),
}

/// Alias for AgentStepType used by streaming module for backward compatibility
pub type StepType = AgentStepType;

/// Final result of agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Final answer from the agent
    pub final_answer: Option<String>,
    /// All execution steps performed
    pub steps: Vec<AgentStep>,
    /// Total execution time
    pub total_duration: std::time::Duration,
    /// Token usage statistics
    pub token_usage: Option<TokenUsage>,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentResult {
    pub fn success(
        final_answer: String,
        steps: Vec<AgentStep>,
        duration: std::time::Duration,
    ) -> Self {
        Self {
            success: true,
            final_answer: Some(final_answer),
            steps,
            total_duration: duration,
            token_usage: None,
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn failure(error: String, steps: Vec<AgentStep>, duration: std::time::Duration) -> Self {
        Self {
            success: false,
            final_answer: None,
            steps,
            total_duration: duration,
            token_usage: None,
            error: Some(error),
            metadata: HashMap::new(),
        }
    }

    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Token usage statistics for tracking AI model consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: usize,
    /// Number of tokens in the completion
    pub completion_tokens: usize,
    /// Total tokens used (prompt + completion)
    pub total_tokens: usize,
}

impl TokenUsage {
    pub fn new(prompt_tokens: usize, completion_tokens: usize) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig::new("test_agent", "A test agent")
            .with_role(AgentRole::Specialist {
                domain: "testing".to_string(),
            })
            .with_capabilities(vec![
                AgentCapability::ToolCalling,
                AgentCapability::Planning,
            ])
            .with_max_steps(20);

        assert_eq!(config.name, "test_agent");
        assert_eq!(config.max_steps, 20);
        assert!(config.has_capability(&AgentCapability::ToolCalling));
        assert!(!config.has_capability(&AgentCapability::Memory));
    }

    #[test]
    fn test_model_config() {
        let config = ModelConfig::new(ModelProvider::OpenAI, "gpt-4").with_parameters(
            ModelParameters::default()
                .with_temperature(0.8)
                .with_max_tokens(2048),
        );

        assert_eq!(config.model_name, "gpt-4");
        assert_eq!(config.parameters.temperature, 0.8);
        assert_eq!(config.parameters.max_tokens, Some(2048));
    }

    #[test]
    fn test_agent_step() {
        let step = AgentStep::new(
            1,
            AgentStepType::ToolCall {
                tool_name: "search".to_string(),
            },
            serde_json::json!({"query": "test"}),
        )
        .with_output(serde_json::json!({"result": "found"}))
        .with_duration(std::time::Duration::from_millis(500));

        assert_eq!(step.step_number, 1);
        assert!(step.is_successful());
        assert_eq!(step.duration, Some(std::time::Duration::from_millis(500)));
    }
}
