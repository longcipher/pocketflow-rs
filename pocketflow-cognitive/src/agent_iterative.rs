//! Iterative cognitive agent node that performs think → plan → reflect loops.
//!
//! This node extends the single-pass CognitiveAgentNode by adding an iterative
//! loop with optional simulated execution. It aims to be closer to lumo-like
//! flows while remaining lightweight and composable.

use std::sync::Arc;

use async_trait::async_trait;
use pocketflow_core::{context::Context, node::Node, state::FlowState};
use pocketflow_mcp::client::McpClient;

use crate::{
    Result,
    context::CognitiveContextExt,
    planning::{GoalOrientedPlanningNode, PlanningConfig, PlanningStrategy},
    thinking::{ChainOfThoughtNode, ThinkingConfig},
    traits::{
        CognitiveNode, ExecutionPlan, ExecutionResult, Goal, PlanningNode, ReasoningChain,
        ThinkingNode,
    },
};

/// Iterative agent that runs think → plan → reflect (+ optional simulated execute) until done.
#[derive(Debug)]
pub struct IterativeCognitiveAgentNode<S: FlowState> {
    name: String,
    thinker: ChainOfThoughtNode<S>,
    planner: GoalOrientedPlanningNode<S>,
    max_iterations: usize,
    target_completion: f64, // 0..=100
    simulate_execution: bool,
    success_state: S,
    error_state: S,
}

impl<S: FlowState> IterativeCognitiveAgentNode<S> {
    pub fn builder() -> IterativeCognitiveAgentNodeBuilder<S> {
        IterativeCognitiveAgentNodeBuilder::new()
    }

    async fn run_thinking(&self, context: &Context) -> Result<ReasoningChain> {
        self.thinker.think(context).await
    }

    async fn run_planning(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan> {
        self.planner.plan(goal, context).await
    }

    async fn evaluate(&self, plan: &ExecutionPlan, context: &Context) -> Result<f64> {
        let progress = self.planner.evaluate_progress(plan, context).await?;
        Ok(progress.completion_percentage)
    }

    fn mark_one_step_completed(&self, plan: &ExecutionPlan, context: &mut Context) -> Result<()> {
        // Simple simulated executor: mark the first pending step as completed
        let mut completed: Vec<String> = context
            .get_json("completed_steps")?
            .unwrap_or_else(Vec::new);

        let completed_set: std::collections::HashSet<_> = completed.iter().cloned().collect();
        if let Some(step) = plan.steps.iter().find(|s| !completed_set.contains(&s.id)) {
            completed.push(step.id.clone());
            context.set("completed_steps", &completed)?;
        }
        Ok(())
    }
}

#[async_trait]
impl<S: FlowState> Node for IterativeCognitiveAgentNode<S> {
    type State = S;

    async fn execute(
        &self,
        mut context: Context,
    ) -> pocketflow_core::error::Result<(Context, Self::State)> {
        // Determine initial goal
        let mut reasoning = match self.run_thinking(&context).await {
            Ok(r) => r,
            Err(e) => {
                context.set("cog_agent_error", format!("thinking failed: {e}"))?;
                return Ok((context, self.error_state.clone()));
            }
        };
        context.set("reasoning_chain", &reasoning)?;
        context.set("reasoning_conclusion", &reasoning.conclusion)?;
        // Persist a thought to cognitive memory
        let _ = context.add_thought(format!("Initial conclusion: {}", reasoning.conclusion));

        let mut goal = if let Some(g) = context.get_json::<Goal>("goal")? {
            g
        } else {
            Goal {
                id: format!("goal_{}", uuid::Uuid::new_v4()),
                description: reasoning.conclusion.clone(),
                success_criteria: vec!["Plan executed".to_string()],
                constraints: vec![],
                priority: 5,
            }
        };

        let mut plan = match self.run_planning(goal.clone(), &context).await {
            Ok(p) => p,
            Err(e) => {
                context.set("cog_agent_error", format!("planning failed: {e}"))?;
                return Ok((context, self.error_state.clone()));
            }
        };
        context.set("execution_goal", &goal)?;
        context.set("execution_plan", &plan)?;

        // Iterate until target completion or max iterations
        for _i in 0..self.max_iterations {
            let completion = self.evaluate(&plan, &context).await.unwrap_or(0.0);
            context.set("plan_completion", completion)?;
            if completion >= self.target_completion {
                return Ok((context, self.success_state.clone()));
            }

            // Optional simulated execution to advance progress
            if self.simulate_execution {
                self.mark_one_step_completed(&plan, &mut context)?;
            }

            // Reflect on current state
            let reflection_input = ExecutionResult {
                success: completion >= self.target_completion,
                output: serde_json::json!({
                    "completion": completion,
                    "plan_id": plan.id,
                }),
                duration: std::time::Duration::from_secs(0),
                errors: vec![],
                metadata: std::collections::HashMap::new(),
            };
            if let Ok(reflection) = self.thinker.reflect(&context, &reflection_input).await {
                // Keep a rolling list of reflections
                let mut reflections: Vec<serde_json::Value> =
                    context.get_json("reflections")?.unwrap_or_default();
                reflections.push(serde_json::to_value(&reflection).unwrap_or_default());
                context.set("reflections", &reflections)?;
                let _ = context.add_thought(
                    reflection
                        .insights
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "Reflected on progress".to_string()),
                );
            }

            // Re-think briefly to refine goal description (lightweight)
            if let Ok(new_reasoning) = self.run_thinking(&context).await {
                reasoning = new_reasoning;
                context.set("reasoning_chain", &reasoning)?;
                context.set("reasoning_conclusion", &reasoning.conclusion)?;
                // Slightly update goal description to steer planning
                goal.description = reasoning.conclusion.clone();
                context.set("execution_goal", &goal)?;
            }

            // Replan if progress is insufficient according to planner policy
            if let Ok(new_plan) = self.planner.replan(&plan, &context).await {
                plan = new_plan;
                context.set("execution_plan", &plan)?;
            }
        }

        // If we exit loop without reaching target
        Ok((context, self.error_state.clone()))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Builder for IterativeCognitiveAgentNode
pub struct IterativeCognitiveAgentNodeBuilder<S: FlowState> {
    name: Option<String>,
    mcp_client: Option<Arc<dyn McpClient>>,
    thinking_config: ThinkingConfig,
    planning_config: PlanningConfig,
    max_iterations: usize,
    target_completion: f64,
    simulate_execution: bool,
    success_state: Option<S>,
    error_state: Option<S>,
}

impl<S: FlowState> IterativeCognitiveAgentNodeBuilder<S> {
    pub fn new() -> Self {
        Self {
            name: None,
            mcp_client: None,
            thinking_config: ThinkingConfig::default(),
            planning_config: PlanningConfig {
                strategy: PlanningStrategy::Sequential,
                ..PlanningConfig::default()
            },
            max_iterations: 10,
            target_completion: 100.0,
            simulate_execution: false,
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
    pub fn max_iterations(mut self, iters: usize) -> Self {
        self.max_iterations = iters;
        self
    }
    pub fn target_completion(mut self, pct: f64) -> Self {
        self.target_completion = pct;
        self
    }
    pub fn simulate_execution(mut self, simulate: bool) -> Self {
        self.simulate_execution = simulate;
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

    pub fn build(self) -> Result<IterativeCognitiveAgentNode<S>> {
        let name = self
            .name
            .unwrap_or_else(|| "iterative_cognitive_agent".to_string());
        let client = self.mcp_client.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("MCP client is required")
        })?;
        let success_state = self.success_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Success state is required")
        })?;
        let error_state = self.error_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Error state is required")
        })?;

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

        Ok(IterativeCognitiveAgentNode {
            name,
            thinker,
            planner,
            max_iterations: self.max_iterations,
            target_completion: self.target_completion,
            simulate_execution: self.simulate_execution,
            success_state,
            error_state,
        })
    }
}

impl<S: FlowState> Default for IterativeCognitiveAgentNodeBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
