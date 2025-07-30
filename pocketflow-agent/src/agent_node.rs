//! Agent node implementation for PocketFlow integration.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use pocketflow_core::prelude::{Context, FlowError, FlowState, Node};
use pocketflow_tools::ToolRegistry;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::{
    agent_types::{AgentConfig, AgentResult, AgentStep, AgentStepType, ModelConfig},
    error::Result,
};

/// Agent states for flow control.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AgentState {
    /// Agent is ready to start
    Ready,
    /// Agent is currently thinking/processing
    Thinking,
    /// Agent is executing a tool
    UsingTool,
    /// Agent is delegating to another agent
    Delegating,
    /// Agent completed successfully
    Success,
    /// Agent encountered an error
    Error,
}

impl FlowState for AgentState {
    fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Success | AgentState::Error)
    }
}

/// Registry for managing multiple AI agents
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, Arc<AgentNode>>>>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an agent with the given name
    pub async fn register(&self, name: String, agent: Arc<AgentNode>) {
        let mut agents = self.agents.write().await;
        agents.insert(name, agent);
    }

    /// Get an agent by name
    pub async fn get(&self, name: &str) -> Option<Arc<AgentNode>> {
        let agents = self.agents.read().await;
        agents.get(name).cloned()
    }

    /// List all registered agent names
    pub async fn list_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }

    /// Remove an agent from the registry
    pub async fn remove(&self, name: &str) -> Option<Arc<AgentNode>> {
        let mut agents = self.agents.write().await;
        agents.remove(name)
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter for managing AI model interactions
#[derive(Debug, Clone)]
pub struct ModelAdapter {
    config: ModelConfig,
}

impl ModelAdapter {
    /// Create a new model adapter
    pub async fn new(config: ModelConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Get the model configuration
    pub fn config(&self) -> &ModelConfig {
        &self.config
    }

    /// Execute a prompt with the configured model
    pub async fn execute_prompt(&self, prompt: &str) -> Result<String> {
        // For now, return a mock response
        // In a real implementation, this would interface with the actual model
        info!("Executing prompt with {} model", self.config.model_name);
        Ok(format!(
            "Response from {}: {}",
            self.config.model_name, prompt
        ))
    }

    /// Check if the model supports tool calling
    pub fn supports_tools(&self) -> bool {
        // Check if the model supports function calling
        matches!(
            self.config.model_name.as_str(),
            "gpt-4" | "gpt-4-turbo" | "gpt-4o" | "gpt-4o-mini" | "gpt-3.5-turbo"
        )
    }
}

/// AI agent node that integrates with PocketFlow workflows.
///
/// This node provides LLM-powered processing capabilities within workflows,
/// enabling intelligent decision making and tool usage.
pub struct AgentNode {
    /// Agent configuration
    pub config: AgentConfig,
    /// Optional tool registry for agent capabilities
    pub tool_registry: Option<Arc<ToolRegistry>>,
    /// Execution history tracking
    pub execution_history: Arc<RwLock<Vec<AgentStep>>>,
}

impl std::fmt::Debug for AgentNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentNode")
            .field("config", &self.config)
            .field("has_tools", &self.tool_registry.is_some())
            .field("steps_count", &"async")
            .finish()
    }
}

impl AgentNode {
    /// Create a new agent node
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            tool_registry: None,
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set tool registry for this agent
    pub fn with_tools(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Execute a single step with the given input
    pub async fn step(&self, input: String) -> Result<AgentResult> {
        let step_number = {
            let history = self.execution_history.read().await;
            history.len() + 1
        };

        info!(
            "Agent '{}' executing step {}",
            self.config.name, step_number
        );

        // Create step entry
        let step = AgentStep {
            step_number,
            step_type: AgentStepType::Thinking,
            input: serde_json::Value::String(input.clone()),
            output: None,
            timestamp: chrono::Utc::now(),
            duration: Some(std::time::Duration::from_millis(0)),
            error: None,
            metadata: std::collections::HashMap::new(),
        };

        // Add to history
        {
            let mut history = self.execution_history.write().await;
            history.push(step);
        }

        // For now, just echo the input with some processing
        let response = format!("Agent {} processed: {}", self.config.name, input);

        // Update step with result
        {
            let mut history = self.execution_history.write().await;
            if let Some(last_step) = history.last_mut() {
                last_step.output = Some(serde_json::Value::String(response.clone()));
                last_step.duration = Some(std::time::Duration::from_millis(100));
            }
        }

        Ok(AgentResult {
            success: true,
            final_answer: Some(response),
            steps: {
                let history = self.execution_history.read().await;
                history.clone()
            },
            total_duration: std::time::Duration::from_millis(100),
            token_usage: None,
            metadata: std::collections::HashMap::new(),
            error: None,
        })
    }

    /// Execute a task with the given input (alias for step method)
    pub async fn execute_task(&self, input: &str) -> Result<AgentResult> {
        self.step(input.to_string()).await
    }

    /// Get execution history
    pub async fn get_history(&self) -> Vec<AgentStep> {
        let history = self.execution_history.read().await;
        history.clone()
    }

    /// Reset agent state
    pub async fn reset(&self) {
        let mut history = self.execution_history.write().await;
        history.clear();
    }
}

#[async_trait]
impl Node for AgentNode {
    type State = AgentState;

    async fn execute(
        &self,
        mut context: Context,
    ) -> std::result::Result<(Context, Self::State), FlowError> {
        // Get input from context
        let input = match context.get_json::<String>("input") {
            Ok(Some(input)) => input,
            _ => "No input provided".to_string(),
        };

        info!(
            "AgentNode '{}' executing with input: {}",
            self.config.name, input
        );

        // Execute agent step
        match self.step(input).await {
            Ok(result) => {
                // Store result in context
                let _ = context.set("agent_result", &result);

                Ok((context, AgentState::Success))
            }
            Err(e) => {
                error!("Agent execution failed: {}", e);
                let _ = context.set("error", e.to_string());
                Ok((context, AgentState::Error))
            }
        }
    }

    fn name(&self) -> String {
        format!("AgentNode({})", self.config.name)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use super::*;
    use crate::agent_types::{
        AgentCapability, AgentRole, ApiConfig, ExecutionMode, ModelConfig, ModelParameters,
        ModelProvider, Priority,
    };

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            id: Uuid::new_v4(),
            name: "test_agent".to_string(),
            description: "A test agent".to_string(),
            role: AgentRole::Independent,
            capabilities: vec![AgentCapability::Basic],
            execution_mode: ExecutionMode::Sync,
            priority: Priority::Normal,
            max_steps: 10,
            timeout: None,
            model_config: ModelConfig {
                provider: ModelProvider::OpenAI,
                model_name: "gpt-4o-mini".to_string(),
                parameters: ModelParameters {
                    temperature: 0.7,
                    max_tokens: Some(1000),
                    top_p: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    stop_sequences: vec![],
                },
                api_config: ApiConfig {
                    api_key: None,
                    base_url: None,
                    timeout: std::time::Duration::from_secs(30),
                    max_retries: 3,
                    retry_delay: std::time::Duration::from_millis(500),
                },
            },
            system_prompt: "You are a test agent".to_string(),
            available_tools: vec![],
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_agent_node_creation() {
        let config = create_test_config();
        let agent = AgentNode::new(config);
        assert_eq!(agent.config.name, "test_agent");
    }

    #[tokio::test]
    async fn test_agent_step_execution() {
        let config = create_test_config();
        let agent = AgentNode::new(config);
        let result = agent.step("Hello, world!".to_string()).await.unwrap();

        assert!(result.success);
        assert!(result.final_answer.as_ref().unwrap().contains("test_agent"));
        assert!(
            result
                .final_answer
                .as_ref()
                .unwrap()
                .contains("Hello, world!")
        );
        assert_eq!(result.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_agent_as_node() {
        let config = create_test_config();
        let agent = AgentNode::new(config);
        let mut context = Context::new();
        let _ = context.set("input", "Test input".to_string());

        let (result_context, state) = agent.execute(context).await.unwrap();

        assert_eq!(state, AgentState::Success);
        assert!(
            result_context
                .get_json::<AgentResult>("agent_result")
                .unwrap()
                .is_some()
        );
    }
}
