//! Goal oriented planning implementations

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use pocketflow_core::{context::Context, node::Node, state::FlowState};
use pocketflow_mcp::client::McpClient;
use serde_json::{Value, json};

use super::{PlanningConfig, PlanningStrategy};
use crate::{
    Result,
    traits::{CognitiveNode, ExecutionPlan, Goal, PlanStep, PlanningNode, ProgressEvaluation},
};

/// A node that performs goal-oriented planning using MCP-connected planning services.
///
/// This node takes high-level goals and decomposes them into actionable execution plans,
/// considering constraints, resources, and success criteria.
pub struct GoalOrientedPlanningNode<S: FlowState> {
    name: String,
    mcp_client: Arc<dyn McpClient>,
    config: PlanningConfig,
    default_goal: Option<Goal>,
    success_state: S,
    error_state: S,
}

impl<S: FlowState> std::fmt::Debug for GoalOrientedPlanningNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoalOrientedPlanningNode")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("default_goal", &self.default_goal)
            .field("success_state", &self.success_state)
            .field("error_state", &self.error_state)
            .finish()
    }
}

impl<S: FlowState> GoalOrientedPlanningNode<S> {
    /// Create a new builder for GoalOrientedPlanningNode
    pub fn builder() -> GoalOrientedPlanningNodeBuilder<S> {
        GoalOrientedPlanningNodeBuilder::new()
    }

    /// Create a new GoalOrientedPlanningNode with the given configuration
    pub fn new(
        name: String,
        mcp_client: Arc<dyn McpClient>,
        config: PlanningConfig,
        default_goal: Option<Goal>,
        success_state: S,
        error_state: S,
    ) -> Self {
        Self {
            name,
            mcp_client,
            config,
            default_goal,
            success_state,
            error_state,
        }
    }

    async fn create_execution_plan(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan> {
        let planning_prompt = self.build_planning_prompt(&goal, context);

        let tool_args = json!({
            "prompt": planning_prompt,
            "max_tokens": 3000,
            "temperature": 0.2,
            "strategy": format!("{:?}", self.config.strategy)
        });

        let response = self
            .mcp_client
            .call_tool("planning_service", tool_args)
            .await
            .map_err(|e| {
                pocketflow_core::error::FlowError::context(format!("Planning failed: {e}"))
            })?;

        self.parse_planning_response(response, goal).await
    }

    fn build_planning_prompt(&self, goal: &Goal, _context: &Context) -> String {
        format!(
            r#"
Create a detailed execution plan for the following goal:

Goal: {}
Description: {}
Success Criteria: {:?}
Constraints: {:?}
Priority: {}

Please provide a structured plan with:
1. Clear steps with dependencies
2. Estimated duration for each step
3. Required tools/resources
4. Risk factors and mitigation strategies
5. Success criteria for each step

Format your response as a structured plan that can be executed step by step.
"#,
            goal.id, goal.description, goal.success_criteria, goal.constraints, goal.priority
        )
    }

    async fn parse_planning_response(&self, response: Value, goal: Goal) -> Result<ExecutionPlan> {
        let response_text = response.as_str().ok_or_else(|| {
            pocketflow_core::error::FlowError::context("Invalid planning response format")
        })?;

        // Simple parsing implementation - in practice, this would be more sophisticated
        let mut steps = Vec::new();
        let mut step_counter = 1;

        for line in response_text.lines() {
            let line = line.trim();
            if line.starts_with(&format!("{step_counter}."))
                || line.starts_with(&format!("Step {step_counter}:"))
            {
                let description = line.split(':').nth(1).unwrap_or(line).trim().to_string();

                steps.push(PlanStep {
                    id: format!("step_{step_counter}"),
                    description,
                    dependencies: vec![], // Could be enhanced to parse dependencies
                    estimated_duration: Duration::from_secs(300), // Default 5 minutes
                    required_tools: vec![], // Could be enhanced to parse required tools
                    success_criteria: vec![], // Could be enhanced to parse success criteria
                });
                step_counter += 1;
            }
        }

        Ok(ExecutionPlan {
            id: format!("plan_{}", uuid::Uuid::new_v4()),
            goal,
            steps: steps.clone(),
            estimated_duration: Duration::from_secs(steps.len() as u64 * 300),
            required_resources: vec![], // Could be enhanced to extract resources
            risk_factors: vec![],       // Could be enhanced to extract risks
        })
    }

    async fn evaluate_plan_progress(
        &self,
        plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ProgressEvaluation> {
        let completed_steps: Vec<String> = context
            .get_json("completed_steps")?
            .unwrap_or_else(Vec::new);

        let total_steps = plan.steps.len();
        let completed_count = completed_steps.len();
        let completion_percentage = if total_steps > 0 {
            (completed_count as f64 / total_steps as f64) * 100.0
        } else {
            0.0
        };

        Ok(ProgressEvaluation {
            completion_percentage,
            completed_steps,
            blocked_steps: vec![], // Could be enhanced to identify blocked steps
            issues_encountered: vec![], // Could be enhanced to identify issues
            recommendations: vec![], // Could be enhanced to provide recommendations
        })
    }
}

#[async_trait]
impl<S: FlowState> Node for GoalOrientedPlanningNode<S> {
    type State = S;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        // Extract or use default goal
        let goal = if let Some(goal_json) = context.get_json::<Goal>("goal")? {
            goal_json
        } else if let Some(default_goal) = &self.default_goal {
            default_goal.clone()
        } else {
            context.set("planning_error", "No goal specified for planning")?;
            return Ok((context, self.error_state.clone()));
        };

        match self.create_execution_plan(goal, &context).await {
            Ok(execution_plan) => {
                // Store the execution plan in context
                context.set("execution_plan", &execution_plan)?;
                context.set("plan_id", &execution_plan.id)?;
                context.set("plan_steps", &execution_plan.steps)?;

                // Evaluate initial progress
                if let Ok(progress) = self.evaluate_plan_progress(&execution_plan, &context).await {
                    context.set("plan_progress", &progress)?;
                }

                Ok((context, self.success_state.clone()))
            }
            Err(e) => {
                context.set("planning_error", e.to_string())?;
                Ok((context, self.error_state.clone()))
            }
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

#[async_trait]
impl<S: FlowState> CognitiveNode for GoalOrientedPlanningNode<S> {
    type CognitiveOutput = ExecutionPlan;

    async fn think(&self, context: &Context) -> Result<Self::CognitiveOutput> {
        let goal = context
            .get_json::<Goal>("goal")?
            .or_else(|| self.default_goal.clone())
            .ok_or_else(|| {
                pocketflow_core::error::FlowError::context("No goal available for planning")
            })?;

        self.create_execution_plan(goal, context).await
    }
}

#[async_trait]
impl<S: FlowState> PlanningNode for GoalOrientedPlanningNode<S> {
    async fn plan(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan> {
        self.create_execution_plan(goal, context).await
    }

    async fn replan(
        &self,
        current_plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ExecutionPlan> {
        // Evaluate current progress
        let progress = self.evaluate_plan_progress(current_plan, context).await?;

        // If progress is below threshold, create a new plan
        if progress.completion_percentage < (self.config.replanning_threshold * 100.0) {
            let updated_goal = Goal {
                id: format!("{}_revised", current_plan.goal.id),
                description: format!("Revised: {}", current_plan.goal.description),
                success_criteria: current_plan.goal.success_criteria.clone(),
                constraints: current_plan.goal.constraints.clone(),
                priority: current_plan.goal.priority,
            };

            self.create_execution_plan(updated_goal, context).await
        } else {
            // Return the current plan if progress is acceptable
            Ok(current_plan.clone())
        }
    }

    async fn evaluate_progress(
        &self,
        plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ProgressEvaluation> {
        self.evaluate_plan_progress(plan, context).await
    }
}

/// Builder for GoalOrientedPlanningNode
pub struct GoalOrientedPlanningNodeBuilder<S: FlowState> {
    name: Option<String>,
    mcp_client: Option<Arc<dyn McpClient>>,
    config: PlanningConfig,
    default_goal: Option<Goal>,
    success_state: Option<S>,
    error_state: Option<S>,
}

impl<S: FlowState> GoalOrientedPlanningNodeBuilder<S> {
    pub fn new() -> Self {
        Self {
            name: None,
            mcp_client: None,
            config: PlanningConfig::default(),
            default_goal: None,
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

    pub fn with_planning_strategy(mut self, strategy: PlanningStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    pub fn with_config(mut self, config: PlanningConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_goal(mut self, goal: Goal) -> Self {
        self.default_goal = Some(goal);
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

    pub fn build(self) -> Result<GoalOrientedPlanningNode<S>> {
        let name = self
            .name
            .unwrap_or_else(|| "goal_oriented_planner".to_string());
        let mcp_client = self.mcp_client.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("MCP client is required")
        })?;
        let success_state = self.success_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Success state is required")
        })?;
        let error_state = self.error_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Error state is required")
        })?;

        Ok(GoalOrientedPlanningNode::new(
            name,
            mcp_client,
            self.config,
            self.default_goal,
            success_state,
            error_state,
        ))
    }
}

impl<S: FlowState> Default for GoalOrientedPlanningNodeBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
