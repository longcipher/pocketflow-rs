//! Adaptive planning implementations

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

/// A node that performs adaptive planning with feedback loops and dynamic replanning.
///
/// This node continuously monitors execution progress and adapts the plan based on
/// real-time feedback, environmental changes, and performance metrics.
pub struct AdaptivePlanningNode<S: FlowState> {
    name: String,
    mcp_client: Arc<dyn McpClient>,
    config: PlanningConfig,
    adaptation_threshold: f64,
    max_adaptations: usize,
    success_state: S,
    error_state: S,
}

impl<S: FlowState> std::fmt::Debug for AdaptivePlanningNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdaptivePlanningNode")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("adaptation_threshold", &self.adaptation_threshold)
            .field("max_adaptations", &self.max_adaptations)
            .field("success_state", &self.success_state)
            .field("error_state", &self.error_state)
            .finish()
    }
}

impl<S: FlowState> AdaptivePlanningNode<S> {
    /// Create a new AdaptivePlanningNode
    pub fn new(
        name: String,
        mcp_client: Arc<dyn McpClient>,
        config: PlanningConfig,
        adaptation_threshold: f64,
        max_adaptations: usize,
        success_state: S,
        error_state: S,
    ) -> Self {
        Self {
            name,
            mcp_client,
            config,
            adaptation_threshold,
            max_adaptations,
            success_state,
            error_state,
        }
    }

    /// Create a new builder for AdaptivePlanningNode
    pub fn builder() -> AdaptivePlanningNodeBuilder<S> {
        AdaptivePlanningNodeBuilder::new()
    }

    async fn create_adaptive_plan(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan> {
        let adaptation_prompt = self.build_adaptive_prompt(&goal, context);

        let tool_args = json!({
            "prompt": adaptation_prompt,
            "max_tokens": 3000,
            "temperature": 0.3, // Slightly higher for adaptability
            "strategy": "adaptive",
            "adaptation_threshold": self.adaptation_threshold,
            "max_adaptations": self.max_adaptations
        });

        let response = self
            .mcp_client
            .call_tool("adaptive_planning_service", tool_args)
            .await
            .map_err(|e| {
                pocketflow_core::error::FlowError::context(format!("Adaptive planning failed: {e}"))
            })?;

        self.parse_adaptive_response(response, goal).await
    }

    fn build_adaptive_prompt(&self, goal: &Goal, context: &Context) -> String {
        let execution_history = context
            .get_json::<Vec<String>>("execution_history")
            .unwrap_or_default()
            .unwrap_or_default();

        let feedback_data = context
            .get_json::<serde_json::Value>("feedback_data")
            .unwrap_or_default()
            .unwrap_or_else(|| json!({}));

        format!(
            r#"
Create an adaptive execution plan for the following goal:

Goal: {}
Description: {}
Success Criteria: {:?}
Constraints: {:?}
Priority: {}

Execution History: {:?}
Feedback Data: {}

This plan should be:
1. Flexible and adaptable to changing conditions
2. Include checkpoints for progress evaluation
3. Have contingency plans for common failure scenarios
4. Support dynamic replanning based on feedback
5. Include metrics for measuring adaptation effectiveness

Adaptation threshold: {:.2}
Maximum adaptations allowed: {}

Format your response as a structured adaptive plan with built-in flexibility points.
"#,
            goal.id,
            goal.description,
            goal.success_criteria,
            goal.constraints,
            goal.priority,
            execution_history,
            feedback_data,
            self.adaptation_threshold,
            self.max_adaptations
        )
    }

    async fn parse_adaptive_response(&self, response: Value, goal: Goal) -> Result<ExecutionPlan> {
        let response_text = response.as_str().ok_or_else(|| {
            pocketflow_core::error::FlowError::context("Invalid adaptive planning response format")
        })?;

        // Enhanced parsing for adaptive plans
        let mut steps = Vec::new();
        let mut step_counter = 1;

        for line in response_text.lines() {
            let line = line.trim();
            if line.starts_with(&format!("{step_counter}."))
                || line.starts_with(&format!("Step {step_counter}:"))
                || line.contains("Checkpoint:")
                || line.contains("Adaptation Point:")
            {
                let description = line.split(':').nth(1).unwrap_or(line).trim().to_string();

                steps.push(PlanStep {
                    id: format!("adaptive_step_{step_counter}"),
                    description,
                    dependencies: vec![], // Could parse dependencies
                    estimated_duration: Duration::from_secs(400), // Slightly longer for adaptation
                    required_tools: vec!["adaptation_monitor".to_string()],
                    success_criteria: vec!["checkpoint_passed".to_string()],
                });
                step_counter += 1;
            }
        }

        Ok(ExecutionPlan {
            id: format!("adaptive_plan_{}", uuid::Uuid::new_v4()),
            goal,
            steps: steps.clone(),
            estimated_duration: Duration::from_secs(steps.len() as u64 * 400),
            required_resources: vec![
                "feedback_monitor".to_string(),
                "adaptation_engine".to_string(),
            ],
            risk_factors: vec!["adaptation_overhead".to_string()],
        })
    }
}

#[async_trait]
impl<S: FlowState> Node for AdaptivePlanningNode<S> {
    type State = S;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        let goal = context.get_json::<Goal>("goal")?.ok_or_else(|| {
            pocketflow_core::error::FlowError::context("No goal specified for adaptive planning")
        })?;

        match self.create_adaptive_plan(goal, &context).await {
            Ok(execution_plan) => {
                context.set("execution_plan", &execution_plan)?;
                context.set("plan_type", "adaptive")?;
                context.set("adaptation_count", 0)?;
                context.set("max_adaptations", self.max_adaptations)?;

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
impl<S: FlowState> CognitiveNode for AdaptivePlanningNode<S> {
    type CognitiveOutput = ExecutionPlan;

    async fn think(&self, context: &Context) -> Result<Self::CognitiveOutput> {
        let goal = context.get_json::<Goal>("goal")?.ok_or_else(|| {
            pocketflow_core::error::FlowError::context("No goal available for adaptive planning")
        })?;

        self.create_adaptive_plan(goal, context).await
    }
}

#[async_trait]
impl<S: FlowState> PlanningNode for AdaptivePlanningNode<S> {
    async fn plan(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan> {
        self.create_adaptive_plan(goal, context).await
    }

    async fn replan(
        &self,
        current_plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ExecutionPlan> {
        let adaptation_count: usize = context.get_json("adaptation_count")?.unwrap_or(0);

        if adaptation_count >= self.max_adaptations {
            return Ok(current_plan.clone());
        }

        // Check if adaptation is needed based on performance metrics
        let progress = self.evaluate_progress(current_plan, context).await?;

        if progress.completion_percentage < (self.adaptation_threshold * 100.0)
            || !progress.issues_encountered.is_empty()
        {
            // Create adapted goal with current context
            let adapted_goal = Goal {
                id: format!("{}_adapted_{}", current_plan.goal.id, adaptation_count + 1),
                description: format!("Adapted: {}", current_plan.goal.description),
                success_criteria: current_plan.goal.success_criteria.clone(),
                constraints: current_plan.goal.constraints.clone(),
                priority: current_plan.goal.priority,
            };

            self.create_adaptive_plan(adapted_goal, context).await
        } else {
            Ok(current_plan.clone())
        }
    }

    async fn evaluate_progress(
        &self,
        plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ProgressEvaluation> {
        let completed_steps: Vec<String> = context.get_json("completed_steps")?.unwrap_or_default();

        let blocked_steps: Vec<String> = context.get_json("blocked_steps")?.unwrap_or_default();

        let issues: Vec<String> = context.get_json("execution_issues")?.unwrap_or_default();

        let total_steps = plan.steps.len();
        let completed_count = completed_steps.len();
        let completion_percentage = if total_steps > 0 {
            (completed_count as f64 / total_steps as f64) * 100.0
        } else {
            0.0
        };

        let mut recommendations = Vec::new();
        if !blocked_steps.is_empty() {
            recommendations.push("Consider alternative approaches for blocked steps".to_string());
        }
        if !issues.is_empty() {
            recommendations.push("Address identified issues before proceeding".to_string());
        }
        if completion_percentage < 50.0 {
            recommendations.push("Consider simplifying the approach".to_string());
        }

        Ok(ProgressEvaluation {
            completion_percentage,
            completed_steps,
            blocked_steps,
            issues_encountered: issues,
            recommendations,
        })
    }
}

/// Builder for AdaptivePlanningNode
pub struct AdaptivePlanningNodeBuilder<S: FlowState> {
    name: Option<String>,
    mcp_client: Option<Arc<dyn McpClient>>,
    config: PlanningConfig,
    adaptation_threshold: f64,
    max_adaptations: usize,
    success_state: Option<S>,
    error_state: Option<S>,
}

impl<S: FlowState> AdaptivePlanningNodeBuilder<S> {
    pub fn new() -> Self {
        Self {
            name: None,
            mcp_client: None,
            config: PlanningConfig {
                strategy: PlanningStrategy::Adaptive,
                ..PlanningConfig::default()
            },
            adaptation_threshold: 0.4,
            max_adaptations: 3,
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

    pub fn with_config(mut self, config: PlanningConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_adaptation_threshold(mut self, threshold: f64) -> Self {
        self.adaptation_threshold = threshold;
        self
    }

    pub fn with_max_adaptations(mut self, max: usize) -> Self {
        self.max_adaptations = max;
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

    pub fn build(self) -> Result<AdaptivePlanningNode<S>> {
        let name = self.name.unwrap_or_else(|| "adaptive_planner".to_string());
        let mcp_client = self.mcp_client.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("MCP client is required")
        })?;
        let success_state = self.success_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Success state is required")
        })?;
        let error_state = self.error_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Error state is required")
        })?;

        Ok(AdaptivePlanningNode::new(
            name,
            mcp_client,
            self.config,
            self.adaptation_threshold,
            self.max_adaptations,
            success_state,
            error_state,
        ))
    }
}

impl<S: FlowState> Default for AdaptivePlanningNodeBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
