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
        // 1) Prefer structured JSON if available, validate minimally when object
        if let Some(obj) = response.as_object() {
            // Minimal plan schema: object with steps array
            let schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": ["string", "null"] },
                    "steps": { "type": "array" },
                    "estimated_duration_seconds": { "type": ["number", "integer", "null"] }
                },
                "required": ["steps"]
            });
            let compiled = jsonschema::Validator::new(&schema).map_err(|e| {
                pocketflow_core::error::FlowError::context(format!(
                    "Planning schema compile error: {e}"
                ))
            })?;
            compiled.validate(&response).map_err(|e| {
                pocketflow_core::error::FlowError::context(format!("Planning JSON invalid: {e}"))
            })?;
            return Ok(self.parse_plan_from_object(obj, goal));
        }

        if let Some(arr) = response.as_array() {
            return Ok(self.parse_plan_from_steps_array(arr, goal));
        }

        // 2) If it's a string, try to parse as JSON string first
        if let Some(s) = response.as_str() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(s) {
                if let Some(obj) = val.as_object() {
                    return Ok(self.parse_plan_from_object(obj, goal));
                } else if let Some(arr) = val.as_array() {
                    return Ok(self.parse_plan_from_steps_array(arr, goal));
                }
            }

            // 3) Fallback: parse numbered lines like "1. ..." or "Step N: ..."
            let mut steps = Vec::new();
            let mut step_counter = 1;
            for line in s.lines() {
                let line = line.trim();
                if line.starts_with(&format!("{step_counter}."))
                    || line.starts_with(&format!("Step {step_counter}:"))
                {
                    let description = line.split(':').nth(1).unwrap_or(line).trim().to_string();
                    steps.push(PlanStep {
                        id: format!("step_{step_counter}"),
                        description,
                        dependencies: vec![],
                        estimated_duration: Duration::from_secs(300),
                        required_tools: vec![],
                        success_criteria: vec![],
                        enforce_success_criteria: None,
                        max_retries: None,
                        initial_backoff_ms: None,
                        stop_on_error: None,
                    });
                    step_counter += 1;
                }
            }

            return Ok(ExecutionPlan {
                id: format!("plan_{}", uuid::Uuid::new_v4()),
                goal,
                steps: steps.clone(),
                estimated_duration: Duration::from_secs(steps.len() as u64 * 300),
                required_resources: vec![],
                risk_factors: vec![],
            });
        }

        Err(pocketflow_core::error::FlowError::context(
            "Invalid planning response format",
        ))
    }

    fn parse_plan_from_object(
        &self,
        obj: &serde_json::Map<String, Value>,
        goal: Goal,
    ) -> ExecutionPlan {
        use std::time::Duration as StdDuration;
        let plan_id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("plan_{}", uuid::Uuid::new_v4()));

        let steps_val = obj
            .get("steps")
            .cloned()
            .unwrap_or_else(|| Value::Array(vec![]));
        let steps = match steps_val {
            Value::Array(arr) => self.collect_steps_from_array(&arr),
            _ => Vec::new(),
        };

        // Plan-level estimated duration (seconds) or compute sum of steps
        let plan_seconds = obj
            .get("estimated_duration_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| steps.iter().map(|s| s.estimated_duration.as_secs()).sum());

        ExecutionPlan {
            id: plan_id,
            goal,
            steps,
            estimated_duration: StdDuration::from_secs(plan_seconds),
            required_resources: obj
                .get("required_resources")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            risk_factors: obj
                .get("risk_factors")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn parse_plan_from_steps_array(&self, arr: &[Value], goal: Goal) -> ExecutionPlan {
        let steps = self.collect_steps_from_array(arr);
        let total = steps.iter().map(|s| s.estimated_duration.as_secs()).sum();
        ExecutionPlan {
            id: format!("plan_{}", uuid::Uuid::new_v4()),
            goal,
            steps,
            estimated_duration: Duration::from_secs(total),
            required_resources: vec![],
            risk_factors: vec![],
        }
    }

    fn collect_steps_from_array(&self, arr: &[Value]) -> Vec<PlanStep> {
        use std::time::Duration as StdDuration;
        arr.iter()
            .enumerate()
            .map(|(i, s)| {
                let description = s
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let id = s
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("step_{}", i + 1));
                let deps: Vec<String> = s
                    .get("dependencies")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| match x {
                                Value::String(st) => Some(st.clone()),
                                Value::Number(n) => Some(format!("step_{n}")),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let duration_secs = s
                    .get("estimated_duration_seconds")
                    .and_then(|v| v.as_u64())
                    .or_else(|| s.get("duration_seconds").and_then(|v| v.as_u64()))
                    .unwrap_or(300);
                let required_tools = s
                    .get("required_tools")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let success_criteria = s
                    .get("success_criteria")
                    .and_then(|v| v.as_array())
                    .map(|a| a.to_vec())
                    .unwrap_or_else(Vec::new);
                PlanStep {
                    id,
                    description,
                    dependencies: deps,
                    estimated_duration: StdDuration::from_secs(duration_secs),
                    required_tools,
                    success_criteria,
                    enforce_success_criteria: s
                        .get("enforce_success_criteria")
                        .and_then(|v| v.as_bool()),
                    max_retries: s
                        .get("max_retries")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    initial_backoff_ms: s.get("initial_backoff_ms").and_then(|v| v.as_u64()),
                    stop_on_error: s.get("stop_on_error").and_then(|v| v.as_bool()),
                }
            })
            .collect()
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
