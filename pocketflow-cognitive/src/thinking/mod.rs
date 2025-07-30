//! Thinking and reasoning node implementations.
//!
//! This module provides concrete implementations of thinking capabilities
//! including chain-of-thought reasoning, reflection, and explanation generation.

use std::sync::Arc;

use async_trait::async_trait;
use pocketflow_core::{context::Context, node::Node, state::FlowState};
use pocketflow_mcp::client::McpClient;
use serde_json::{Value, json};

use crate::{
    Result,
    traits::{
        CognitiveNode, Decision, ExecutionResult, Explanation, ReasoningChain, ReasoningStep,
        Reflection, ThinkingNode,
    },
};

pub mod chain_of_thought;
pub mod explanation;
pub mod reflection;

/// Strategy for conducting reasoning processes
#[derive(Debug, Clone)]
pub enum ReasoningStrategy {
    /// Step-by-step logical reasoning
    StepByStep,
    /// Problem decomposition approach
    Decomposition,
    /// Analogical reasoning using similar cases
    Analogical,
    /// Critical thinking with explicit evaluation
    Critical,
    /// Creative thinking for novel solutions
    Creative,
}

/// Configuration for thinking operations
#[derive(Debug, Clone)]
pub struct ThinkingConfig {
    pub strategy: ReasoningStrategy,
    pub max_reasoning_steps: usize,
    pub confidence_threshold: f64,
    pub enable_reflection: bool,
    pub enable_explanation: bool,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            strategy: ReasoningStrategy::StepByStep,
            max_reasoning_steps: 10,
            confidence_threshold: 0.7,
            enable_reflection: true,
            enable_explanation: true,
        }
    }
}

/// A node that performs chain-of-thought reasoning using MCP-connected LLM services.
///
/// This node breaks down complex problems into step-by-step reasoning chains,
/// making the thinking process transparent and debuggable.
pub struct ChainOfThoughtNode<S: FlowState> {
    name: String,
    mcp_client: Arc<dyn McpClient>,
    config: ThinkingConfig,
    prompt_template: String,
    success_state: S,
    error_state: S,
}

impl<S: FlowState> std::fmt::Debug for ChainOfThoughtNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainOfThoughtNode")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("prompt_template", &self.prompt_template)
            .field("success_state", &self.success_state)
            .field("error_state", &self.error_state)
            .finish()
    }
}

impl<S: FlowState> ChainOfThoughtNode<S> {
    /// Create a new builder for ChainOfThoughtNode
    pub fn builder() -> ChainOfThoughtNodeBuilder<S> {
        ChainOfThoughtNodeBuilder::new()
    }

    /// Create a new ChainOfThoughtNode with the given configuration
    pub fn new(
        name: String,
        mcp_client: Arc<dyn McpClient>,
        config: ThinkingConfig,
        success_state: S,
        error_state: S,
    ) -> Self {
        Self {
            name,
            mcp_client,
            config,
            prompt_template: Self::default_prompt_template(),
            success_state,
            error_state,
        }
    }

    fn default_prompt_template() -> String {
        r#"
Think through this step by step:

Problem: {problem}
Context: {context}

Please provide your reasoning in the following format:
Step 1: [Your first thought and reasoning]
Step 2: [Your second thought, building on step 1]
...
Conclusion: [Your final conclusion]

Be explicit about your reasoning process and show your work.
"#
        .to_string()
    }

    async fn perform_reasoning(&self, problem: &str, _context: &Context) -> Result<ReasoningChain> {
        let prompt = self
            .prompt_template
            .replace("{problem}", problem)
            .replace("{context}", "{}"); // Simplified context placeholder

        let tool_args = json!({
            "prompt": prompt,
            "max_tokens": 2000,
            "temperature": 0.3
        });

        let response = self
            .mcp_client
            .call_tool("llm_reasoning", tool_args)
            .await
            .map_err(|e| {
                pocketflow_core::error::FlowError::context(format!("Reasoning failed: {e}"))
            })?;

        self.parse_reasoning_response(response).await
    }

    async fn parse_reasoning_response(&self, response: Value) -> Result<ReasoningChain> {
        let response_text = response.as_str().ok_or_else(|| {
            pocketflow_core::error::FlowError::context("Invalid reasoning response format")
        })?;

        let mut steps = Vec::new();
        let mut step_number = 1;
        let mut conclusion = String::new();

        for line in response_text.lines() {
            let line = line.trim();
            if line.starts_with(&format!("Step {step_number}:")) {
                let thought = line
                    .strip_prefix(&format!("Step {step_number}:"))
                    .unwrap_or(line)
                    .trim()
                    .to_string();

                steps.push(ReasoningStep {
                    step_number,
                    thought: thought.clone(),
                    evidence: vec![], // Could be enhanced to extract evidence
                    inference: thought,
                    confidence: 0.8, // Could be enhanced with confidence extraction
                });
                step_number += 1;
            } else if line.starts_with("Conclusion:") {
                conclusion = line
                    .strip_prefix("Conclusion:")
                    .unwrap_or(line)
                    .trim()
                    .to_string();
            }
        }

        Ok(ReasoningChain {
            steps,
            conclusion,
            confidence: 0.8, // Could be enhanced with overall confidence calculation
            alternatives_considered: vec![], // Could be enhanced to extract alternatives
        })
    }
}

#[async_trait]
impl<S: FlowState> Node for ChainOfThoughtNode<S> {
    type State = S;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        // Extract the problem to reason about from context
        let problem = context
            .get_json::<String>("problem")?
            .unwrap_or_else(|| "No specific problem defined".to_string());

        match self.perform_reasoning(&problem, &context).await {
            Ok(reasoning_chain) => {
                // Store the reasoning chain in context
                context.set("reasoning_chain", &reasoning_chain)?;
                context.set("reasoning_conclusion", &reasoning_chain.conclusion)?;
                context.set("reasoning_confidence", reasoning_chain.confidence)?;

                // Optionally perform reflection if enabled
                if self.config.enable_reflection
                    && let Ok(reflection) = self
                        .reflect(
                            &context,
                            &ExecutionResult {
                                success: true,
                                output: json!(reasoning_chain.conclusion),
                                duration: std::time::Duration::from_secs(0),
                                errors: vec![],
                                metadata: std::collections::HashMap::new(),
                            },
                        )
                        .await
                {
                    context.set("reasoning_reflection", &reflection)?;
                }

                Ok((context, self.success_state.clone()))
            }
            Err(e) => {
                context.set("reasoning_error", e.to_string())?;
                Ok((context, self.error_state.clone()))
            }
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

#[async_trait]
impl<S: FlowState> CognitiveNode for ChainOfThoughtNode<S> {
    type CognitiveOutput = ReasoningChain;

    async fn think(&self, context: &Context) -> Result<Self::CognitiveOutput> {
        let problem = context
            .get_json::<String>("problem")?
            .unwrap_or_else(|| "General reasoning task".to_string());

        self.perform_reasoning(&problem, context).await
    }
}

#[async_trait]
impl<S: FlowState> ThinkingNode for ChainOfThoughtNode<S> {
    async fn reason(&self, context: &Context) -> Result<ReasoningChain> {
        self.think(context).await
    }

    async fn reflect(
        &self,
        _context: &Context,
        previous_result: &ExecutionResult,
    ) -> Result<Reflection> {
        let reflection_prompt = format!(
            "Reflect on this reasoning process and result:\n\nResult: {previous_result:?}\n\nProvide insights, identify any errors, and suggest improvements."
        );

        let tool_args = json!({
            "prompt": reflection_prompt,
            "max_tokens": 1000,
            "temperature": 0.4
        });

        let response = self
            .mcp_client
            .call_tool("llm_reflection", tool_args)
            .await
            .map_err(|e| {
                pocketflow_core::error::FlowError::context(format!("Reflection failed: {e}"))
            })?;

        // Parse reflection response (simplified)
        let reflection_text = response.as_str().unwrap_or("Unable to generate reflection");

        Ok(Reflection {
            insights: vec![reflection_text.to_string()],
            identified_errors: vec![],
            improvement_suggestions: vec![],
            confidence_in_result: 0.7,
        })
    }

    async fn explain(&self, _context: &Context, decision: &Decision) -> Result<Explanation> {
        let explanation_prompt = format!(
            "Explain this decision:\n\nDecision Point: {}\nChosen Option: {}\nAvailable Options: {:?}",
            decision.decision_point, decision.chosen_option, decision.available_options
        );

        let tool_args = json!({
            "prompt": explanation_prompt,
            "max_tokens": 800,
            "temperature": 0.3
        });

        let response = self
            .mcp_client
            .call_tool("llm_explanation", tool_args)
            .await
            .map_err(|e| {
                pocketflow_core::error::FlowError::context(format!("Explanation failed: {e}"))
            })?;

        let explanation_text = response
            .as_str()
            .unwrap_or("Unable to generate explanation");

        Ok(Explanation {
            reasoning: explanation_text.to_string(),
            key_factors: vec![],
            alternatives_rejected: vec![],
            confidence_level: 0.8,
        })
    }
}

/// Builder for ChainOfThoughtNode
pub struct ChainOfThoughtNodeBuilder<S: FlowState> {
    name: Option<String>,
    mcp_client: Option<Arc<dyn McpClient>>,
    config: ThinkingConfig,
    prompt_template: Option<String>,
    success_state: Option<S>,
    error_state: Option<S>,
}

impl<S: FlowState> ChainOfThoughtNodeBuilder<S> {
    pub fn new() -> Self {
        Self {
            name: None,
            mcp_client: None,
            config: ThinkingConfig::default(),
            prompt_template: None,
            success_state: None,
            error_state: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_mcp_client(mut self, client: Arc<dyn McpClient>) -> Self {
        self.mcp_client = Some(client);
        self
    }

    pub fn with_reasoning_strategy(mut self, strategy: ReasoningStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    pub fn with_config(mut self, config: ThinkingConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_prompt_template(mut self, template: impl Into<String>) -> Self {
        self.prompt_template = Some(template.into());
        self
    }

    pub fn on_success(mut self, state: S) -> Self {
        self.success_state = Some(state);
        self
    }

    pub fn on_error(mut self, state: S) -> Self {
        self.error_state = Some(state);
        self
    }

    pub fn build(self) -> Result<ChainOfThoughtNode<S>> {
        let name = self.name.unwrap_or_else(|| "chain_of_thought".to_string());
        let mcp_client = self.mcp_client.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("MCP client is required")
        })?;
        let success_state = self.success_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Success state is required")
        })?;
        let error_state = self.error_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Error state is required")
        })?;

        let mut node =
            ChainOfThoughtNode::new(name, mcp_client, self.config, success_state, error_state);

        if let Some(template) = self.prompt_template {
            node.prompt_template = template;
        }

        Ok(node)
    }
}

impl<S: FlowState> Default for ChainOfThoughtNodeBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
