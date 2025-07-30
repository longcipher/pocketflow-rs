use std::{sync::Arc, time::Duration};

use crate::{
    agent_node::{AgentNode, AgentRegistry},
    agent_types::{
        AgentCapability, AgentConfig, AgentRole, ApiConfig, ExecutionMode, ModelConfig,
        ModelParameters, ModelProvider, Priority,
    },
    error::Result,
};

/// Builder for creating AgentNode instances
pub struct AgentNodeBuilder {
    config: AgentConfig,
    model_config: Option<ModelConfig>,
    tool_registry: Option<Arc<pocketflow_tools::ToolRegistry>>,
    agent_registry: Option<Arc<AgentRegistry>>,
}

impl AgentNodeBuilder {
    /// Create a new builder with basic configuration
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            config: AgentConfig::new(name, description),
            model_config: None,
            tool_registry: None,
            agent_registry: None,
        }
    }

    /// Set the agent role
    pub fn with_role(mut self, role: AgentRole) -> Self {
        self.config = self.config.with_role(role);
        self
    }

    /// Set the agent capabilities
    pub fn with_capabilities(mut self, capabilities: Vec<AgentCapability>) -> Self {
        self.config = self.config.with_capabilities(capabilities);
        self
    }

    /// Set the execution mode
    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.config = self.config.with_execution_mode(mode);
        self
    }

    /// Set the priority level
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.config = self.config.with_priority(priority);
        self
    }

    /// Set maximum execution steps
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.config = self.config.with_max_steps(max_steps);
        self
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config = self.config.with_timeout(timeout);
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config = self.config.with_system_prompt(prompt);
        self
    }

    /// Set available tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.config = self.config.with_tools(tools);
        self
    }

    /// Add a single tool
    pub fn add_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.config.available_tools.push(tool_name.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config = self.config.with_metadata(key, value);
        self
    }

    /// Set the model configuration
    pub fn with_model_config(mut self, config: ModelConfig) -> Self {
        self.model_config = Some(config);
        self
    }

    /// Set model using provider and name
    pub fn with_model(mut self, provider: ModelProvider, model_name: impl Into<String>) -> Self {
        self.model_config = Some(ModelConfig::new(provider, model_name));
        self
    }

    /// Set OpenAI model
    pub fn with_openai_model(self, model_name: impl Into<String>) -> Self {
        self.with_model(ModelProvider::OpenAI, model_name)
    }

    /// Set Anthropic model
    pub fn with_anthropic_model(self, model_name: impl Into<String>) -> Self {
        self.with_model(ModelProvider::Anthropic, model_name)
    }

    /// Set Google model
    pub fn with_google_model(self, model_name: impl Into<String>) -> Self {
        self.with_model(ModelProvider::Google, model_name)
    }

    /// Set Ollama model
    pub fn with_ollama_model(self, model_name: impl Into<String>) -> Self {
        self.with_model(ModelProvider::Ollama, model_name)
    }

    /// Set model parameters
    pub fn with_parameters(self, parameters: ModelParameters) -> Self {
        self.with_model_config(ModelConfig {
            parameters,
            ..Default::default()
        })
    }

    /// Set model temperature
    pub fn with_temperature(self, temperature: f32) -> Self {
        let params = self
            .model_config
            .as_ref()
            .map(|c| c.parameters.clone())
            .unwrap_or_default()
            .with_temperature(temperature);
        self.with_parameters(params)
    }

    /// Set max tokens
    pub fn with_max_tokens(self, max_tokens: usize) -> Self {
        let params = self
            .model_config
            .as_ref()
            .map(|c| c.parameters.clone())
            .unwrap_or_default()
            .with_max_tokens(max_tokens);
        self.with_parameters(params)
    }

    /// Configure API settings
    pub fn with_api_config(self, api_config: ApiConfig) -> Self {
        self.with_model_config(ModelConfig {
            api_config,
            ..Default::default()
        })
    }

    /// Set API key
    pub fn with_api_key(self, api_key: impl Into<String>) -> Self {
        let api_config = self
            .model_config
            .as_ref()
            .map(|c| c.api_config.clone())
            .unwrap_or_default()
            .with_api_key(api_key);
        self.with_api_config(api_config)
    }

    /// Set base URL
    pub fn with_base_url(self, base_url: impl Into<String>) -> Self {
        let api_config = self
            .model_config
            .as_ref()
            .map(|c| c.api_config.clone())
            .unwrap_or_default()
            .with_base_url(base_url);
        self.with_api_config(api_config)
    }

    /// Set tool registry
    pub fn with_tool_registry(mut self, registry: Arc<pocketflow_tools::ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Set agent registry for delegation
    pub fn with_agent_registry(mut self, registry: Arc<AgentRegistry>) -> Self {
        self.agent_registry = Some(registry);
        self
    }

    /// Build the AgentNode
    pub async fn build(mut self) -> Result<AgentNode> {
        // Use provided model config or default
        let model_config = self
            .model_config
            .unwrap_or_else(|| ModelConfig::new(ModelProvider::OpenAI, "gpt-4o-mini"));

        // Update agent config with model config
        self.config.model_config = model_config.clone();

        // Create agent node
        let mut agent = AgentNode::new(self.config);

        // Add tool registry if provided
        if let Some(tool_registry) = self.tool_registry {
            agent = agent.with_tools(tool_registry);
        }

        // Note: agent_registry could be used for multi-agent coordination in the future
        // For now, we'll just create the agent without it

        Ok(agent)
    }
}

/// Builder for ModelConfig
pub struct ModelConfigBuilder {
    provider: ModelProvider,
    model_name: String,
    parameters: ModelParameters,
    api_config: ApiConfig,
}

impl ModelConfigBuilder {
    pub fn new(provider: ModelProvider, model_name: impl Into<String>) -> Self {
        Self {
            provider,
            model_name: model_name.into(),
            parameters: ModelParameters::default(),
            api_config: ApiConfig::default(),
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.parameters.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.parameters.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.parameters.top_p = Some(top_p);
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_config.api_key = Some(api_key.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_config.base_url = Some(base_url.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.api_config.timeout = timeout;
        self
    }

    pub fn build(self) -> ModelConfig {
        ModelConfig {
            provider: self.provider,
            model_name: self.model_name,
            parameters: self.parameters,
            api_config: self.api_config,
        }
    }
}

/// Preset configurations for common use cases
pub struct AgentPresets;

impl AgentPresets {
    /// Create a basic function-calling agent
    pub fn function_calling_agent(name: impl Into<String>) -> AgentNodeBuilder {
        AgentNodeBuilder::new(
            name,
            "An AI agent capable of calling tools to complete tasks",
        )
        .with_capabilities(vec![AgentCapability::Basic, AgentCapability::ToolCalling])
        .with_role(AgentRole::Independent)
        .with_system_prompt(
            "You are a helpful AI assistant that can use tools to complete tasks. \
                When you need to use a tool, respond with JSON in this format: \
                {\"tool_call\": \"tool_name\", \"parameters\": {...}}. \
                When you have a final answer, respond with: Final Answer: [your answer]",
        )
        .with_max_steps(15)
        .with_temperature(0.7)
    }

    /// Create a coding specialist agent
    pub fn code_agent(name: impl Into<String>) -> AgentNodeBuilder {
        AgentNodeBuilder::new(name, "An AI agent specialized in code analysis and generation")
            .with_capabilities(vec![
                AgentCapability::Basic,
                AgentCapability::ToolCalling,
                AgentCapability::CodeExecution,
            ])
            .with_role(AgentRole::Specialist { domain: "coding".to_string() })
            .with_system_prompt(
                "You are an expert software engineer. You can analyze code, write programs, \
                debug issues, and use development tools. Always provide working, well-commented code."
            )
            .with_max_steps(20)
            .with_temperature(0.3)
    }

    /// Create a research agent
    pub fn research_agent(name: impl Into<String>) -> AgentNodeBuilder {
        AgentNodeBuilder::new(name, "An AI agent specialized in research and information gathering")
            .with_capabilities(vec![
                AgentCapability::Basic,
                AgentCapability::ToolCalling,
            ])
            .with_role(AgentRole::Specialist { domain: "research".to_string() })
            .with_system_prompt(
                "You are a research specialist. You excel at finding, analyzing, and synthesizing \
                information from various sources. Use tools to gather data and provide comprehensive, \
                well-sourced answers."
            )
            .with_max_steps(25)
            .with_temperature(0.5)
    }

    /// Create a coordinator agent for multi-agent workflows
    pub fn coordinator_agent(name: impl Into<String>) -> AgentNodeBuilder {
        AgentNodeBuilder::new(name, "An AI agent that coordinates other agents")
            .with_capabilities(vec![
                AgentCapability::Basic,
                AgentCapability::Coordination,
                AgentCapability::Planning,
            ])
            .with_role(AgentRole::Coordinator)
            .with_system_prompt(
                "You are a team coordinator. Your role is to break down complex tasks, \
                delegate subtasks to appropriate specialist agents, and synthesize their \
                results into a final answer. To delegate, use: \
                delegate to \"agent_name\" task: \"specific task description\"",
            )
            .with_max_steps(30)
            .with_temperature(0.6)
    }

    /// Create a planning agent
    pub fn planning_agent(name: impl Into<String>) -> AgentNodeBuilder {
        AgentNodeBuilder::new(name, "An AI agent specialized in planning and strategy")
            .with_capabilities(vec![
                AgentCapability::Basic,
                AgentCapability::Planning,
                AgentCapability::ToolCalling,
            ])
            .with_role(AgentRole::Specialist { domain: "planning".to_string() })
            .with_system_prompt(
                "You are a strategic planning expert. You excel at breaking down complex goals \
                into actionable steps, identifying dependencies, and creating detailed execution plans."
            )
            .with_max_steps(20)
            .with_temperature(0.4)
    }
}

/// Quick creation functions
impl AgentNodeBuilder {
    /// Quick function-calling agent
    pub fn function_calling(name: impl Into<String>) -> Self {
        AgentPresets::function_calling_agent(name)
    }

    /// Quick code agent
    pub fn code_specialist(name: impl Into<String>) -> Self {
        AgentPresets::code_agent(name)
    }

    /// Quick research agent
    pub fn researcher(name: impl Into<String>) -> Self {
        AgentPresets::research_agent(name)
    }

    /// Quick coordinator agent
    pub fn coordinator(name: impl Into<String>) -> Self {
        AgentPresets::coordinator_agent(name)
    }

    /// Quick planning agent
    pub fn planner(name: impl Into<String>) -> Self {
        AgentPresets::planning_agent(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_builder_basic() {
        let agent = AgentNodeBuilder::new("test", "A test agent")
            .with_role(AgentRole::Independent)
            .with_max_steps(5)
            .with_temperature(0.8)
            .build()
            .await
            .unwrap();

        assert_eq!(agent.name(), "test");
        assert_eq!(agent.config().max_steps, 5);
        assert_eq!(agent.config().model_config.parameters.temperature, 0.8);
    }

    #[tokio::test]
    async fn test_preset_function_calling() {
        let agent = AgentPresets::function_calling_agent("function_caller")
            .build()
            .await
            .unwrap();

        assert!(agent.config().has_capability(&AgentCapability::ToolCalling));
        assert_eq!(agent.config().max_steps, 15);
    }

    #[test]
    fn test_model_config_builder() {
        let config = ModelConfigBuilder::new(ModelProvider::OpenAI, "gpt-4")
            .with_temperature(0.9)
            .with_max_tokens(2048)
            .with_api_key("test-key")
            .build();

        assert_eq!(config.model_name, "gpt-4");
        assert_eq!(config.parameters.temperature, 0.9);
        assert_eq!(config.parameters.max_tokens, Some(2048));
        assert_eq!(config.api_config.api_key, Some("test-key".to_string()));
    }

    #[tokio::test]
    async fn test_quick_builders() {
        let _code_agent = AgentNodeBuilder::code_specialist("coder")
            .build()
            .await
            .unwrap();
        let _researcher = AgentNodeBuilder::researcher("finder")
            .build()
            .await
            .unwrap();
        let _coordinator = AgentNodeBuilder::coordinator("boss").build().await.unwrap();
        let _planner = AgentNodeBuilder::planner("strategist")
            .build()
            .await
            .unwrap();
    }
}
