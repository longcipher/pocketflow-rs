//! Unified cognitive agent node that performs thinking (reasoning) + planning.
//!
//! This node is inspired by approaches like Lumo: it first reasons about the
//! problem (thinking), then produces a plan (planning), and writes both into
//! the workflow Context for downstream nodes to use or for immediate execution.

use std::sync::Arc;

use async_trait::async_trait;
use pocketflow_core::{context::Context, node::Node, state::FlowState};
use pocketflow_mcp::client::McpClient;

use crate::{
    Result,
    planning::{GoalOrientedPlanningNode, PlanningConfig, PlanningStrategy},
    thinking::{ChainOfThoughtNode, ThinkingConfig},
    traits::{CognitiveNode, ExecutionPlan, Goal, PlanningNode, ReasoningChain},
};

/// A single node that coordinates thinking then planning.
#[derive(Debug)]
pub struct CognitiveAgentNode<S: FlowState> {
    name: String,
    thinker: ChainOfThoughtNode<S>,
    planner: GoalOrientedPlanningNode<S>,
    success_state: S,
    error_state: S,
}

impl<S: FlowState> CognitiveAgentNode<S> {
    pub fn builder() -> CognitiveAgentNodeBuilder<S> {
        CognitiveAgentNodeBuilder::new()
    }

    async fn run_thinking(&self, context: &Context) -> Result<ReasoningChain> {
        self.thinker.think(context).await
    }

    async fn run_planning(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan> {
        self.planner.plan(goal, context).await
    }
}

#[async_trait]
impl<S: FlowState> Node for CognitiveAgentNode<S> {
    type State = S;

    async fn execute(
        &self,
        mut context: Context,
    ) -> pocketflow_core::error::Result<(Context, Self::State)> {
        // 1) Thinking: get or derive the problem, produce a reasoning chain
        let reasoning = match self.run_thinking(&context).await {
            Ok(r) => r,
            Err(e) => {
                context.set("cog_agent_error", format!("thinking failed: {e}"))?;
                return Ok((context, self.error_state.clone()));
            }
        };
        context.set("reasoning_chain", &reasoning)?;
        context.set("reasoning_conclusion", &reasoning.conclusion)?;

        // 2) Planning: build or reuse a goal, then plan
        let goal = if let Some(goal) = context.get_json::<Goal>("goal")? {
            goal
        } else {
            // Synthesize a simple goal from reasoning conclusion
            Goal {
                id: format!("goal_{}", uuid::Uuid::new_v4()),
                description: reasoning.conclusion.clone(),
                success_criteria: vec!["Plan executed".to_string()],
                constraints: vec![],
                priority: 5,
            }
        };

        let plan = match self.run_planning(goal.clone(), &context).await {
            Ok(p) => p,
            Err(e) => {
                context.set("cog_agent_error", format!("planning failed: {e}"))?;
                return Ok((context, self.error_state.clone()));
            }
        };

        context.set("execution_goal", &goal)?;
        context.set("execution_plan", &plan)?;

        Ok((context, self.success_state.clone()))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Builder for CognitiveAgentNode
pub struct CognitiveAgentNodeBuilder<S: FlowState> {
    name: Option<String>,
    mcp_client: Option<Arc<dyn McpClient>>,
    thinking_config: ThinkingConfig,
    planning_config: PlanningConfig,
    success_state: Option<S>,
    error_state: Option<S>,
}

impl<S: FlowState> CognitiveAgentNodeBuilder<S> {
    pub fn new() -> Self {
        Self {
            name: None,
            mcp_client: None,
            thinking_config: ThinkingConfig::default(),
            planning_config: PlanningConfig {
                strategy: PlanningStrategy::Sequential,
                ..PlanningConfig::default()
            },
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

    pub fn with_thinking_config(mut self, cfg: ThinkingConfig) -> Self {
        self.thinking_config = cfg;
        self
    }

    pub fn with_planning_config(mut self, cfg: PlanningConfig) -> Self {
        self.planning_config = cfg;
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

    pub fn build(self) -> Result<CognitiveAgentNode<S>> {
        let name = self.name.unwrap_or_else(|| "cognitive_agent".to_string());
        let client = self.mcp_client.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("MCP client is required")
        })?;
        let success_state = self.success_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Success state is required")
        })?;
        let error_state = self.error_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Error state is required")
        })?;

        // Reuse existing nodes inside
        let thinker = ChainOfThoughtNode::builder()
            .name(format!("{name}_thinker"))
            .with_mcp_client(client.clone())
            .with_config(self.thinking_config)
            .on_success(success_state.clone())
            .on_error(error_state.clone())
            .build()?;

        let planner = GoalOrientedPlanningNode::builder()
            .name(format!("{name}_planner"))
            .with_mcp_client(client)
            .with_config(self.planning_config)
            .on_success(success_state.clone())
            .on_error(error_state.clone())
            .build()?;

        Ok(CognitiveAgentNode {
            name,
            thinker,
            planner,
            success_state,
            error_state,
        })
    }
}

impl<S: FlowState> Default for CognitiveAgentNodeBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
