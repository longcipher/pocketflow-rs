//! Hierarchical planning implementations

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use pocketflow_core::{context::Context, node::Node, state::FlowState};
use pocketflow_mcp::client::McpClient;
use serde_json::{json, Value};

use super::{PlanningConfig, PlanningStrategy};
use crate::{
    traits::{CognitiveNode, ExecutionPlan, Goal, PlanStep, PlanningNode, ProgressEvaluation},
    Result,
};

/// A node that performs hierarchical planning with task decomposition.
///
/// This node breaks down complex goals into hierarchical sub-goals and sub-tasks,
/// creating multi-level execution plans with proper dependency management.
pub struct HierarchicalPlanningNode<S: FlowState> {
    name: String,
    mcp_client: Arc<dyn McpClient>,
    config: PlanningConfig,
    max_depth: usize,
    decomposition_threshold: usize,
    success_state: S,
    error_state: S,
}

impl<S: FlowState> std::fmt::Debug for HierarchicalPlanningNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HierarchicalPlanningNode")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("max_depth", &self.max_depth)
            .field("decomposition_threshold", &self.decomposition_threshold)
            .field("success_state", &self.success_state)
            .field("error_state", &self.error_state)
            .finish()
    }
}

/// Represents a hierarchical task in the planning structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HierarchicalTask {
    pub id: String,
    pub goal: Goal,
    pub level: usize,
    pub parent_id: Option<String>,
    pub sub_tasks: Vec<HierarchicalTask>,
    pub plan_steps: Vec<PlanStep>,
}

impl<S: FlowState> HierarchicalPlanningNode<S> {
    /// Create a new HierarchicalPlanningNode
    pub fn new(
        name: String,
        mcp_client: Arc<dyn McpClient>,
        config: PlanningConfig,
        max_depth: usize,
        decomposition_threshold: usize,
        success_state: S,
        error_state: S,
    ) -> Self {
        Self {
            name,
            mcp_client,
            config,
            max_depth,
            decomposition_threshold,
            success_state,
            error_state,
        }
    }

    /// Create a new builder for HierarchicalPlanningNode
    pub fn builder() -> HierarchicalPlanningNodeBuilder<S> {
        HierarchicalPlanningNodeBuilder::new()
    }

    async fn create_hierarchical_plan(
        &self,
        goal: Goal,
        context: &Context,
    ) -> Result<ExecutionPlan> {
        let root_task = self.decompose_goal(goal.clone(), 0, None, context).await?;
        let flattened_steps = self.flatten_hierarchical_tasks(&root_task);

        Ok(ExecutionPlan {
            id: format!("hierarchical_plan_{}", uuid::Uuid::new_v4()),
            goal,
            steps: flattened_steps.clone(),
            estimated_duration: Duration::from_secs(flattened_steps.len() as u64 * 350),
            required_resources: vec![
                "task_decomposer".to_string(),
                "dependency_manager".to_string(),
            ],
            risk_factors: vec![
                "complexity_overhead".to_string(),
                "coordination_challenges".to_string(),
            ],
        })
    }

    fn decompose_goal<'a>(
        &'a self,
        goal: Goal,
        current_depth: usize,
        parent_id: Option<String>,
        context: &'a Context,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HierarchicalTask>> + Send + 'a>>
    {
        Box::pin(async move {
            if current_depth >= self.max_depth {
                // Create leaf task with direct plan steps
                let steps = self.create_leaf_task_steps(&goal).await?;
                return Ok(HierarchicalTask {
                    id: format!("task_{}_{}", current_depth, uuid::Uuid::new_v4()),
                    goal,
                    level: current_depth,
                    parent_id,
                    sub_tasks: vec![],
                    plan_steps: steps,
                });
            }

            let decomposition_prompt =
                self.build_decomposition_prompt(&goal, current_depth, context);

            let tool_args = json!({
                "prompt": decomposition_prompt,
                "max_tokens": 2000,
                "temperature": 0.2,
                "strategy": "hierarchical_decomposition",
                "max_depth": self.max_depth,
                "current_depth": current_depth
            });

            let response = self
                .mcp_client
                .call_tool("hierarchical_planning_service", tool_args)
                .await
                .map_err(|e| {
                    pocketflow_core::error::FlowError::context(format!(
                        "Hierarchical planning failed: {e}"
                    ))
                })?;

            self.parse_decomposition_response(response, goal, current_depth, parent_id, context)
                .await
        })
    }

    fn build_decomposition_prompt(
        &self,
        goal: &Goal,
        current_depth: usize,
        _context: &Context,
    ) -> String {
        format!(
            r#"
Decompose the following goal into hierarchical sub-goals:

Goal: {}
Description: {}
Success Criteria: {:?}
Constraints: {:?}
Priority: {}
Current Depth: {}
Max Depth: {}

Please break this goal down into 2-5 meaningful sub-goals that:
1. Are smaller and more manageable than the parent goal
2. When completed together, achieve the parent goal
3. Have clear dependencies and ordering
4. Can be further decomposed if needed

For each sub-goal, provide:
- A clear description
- Success criteria
- Dependencies on other sub-goals
- Estimated complexity (1-10)

Format your response as a structured list of sub-goals.
"#,
            goal.id,
            goal.description,
            goal.success_criteria,
            goal.constraints,
            goal.priority,
            current_depth,
            self.max_depth
        )
    }

    fn parse_decomposition_response<'a>(
        &'a self,
        response: Value,
        goal: Goal,
        current_depth: usize,
        parent_id: Option<String>,
        context: &'a Context,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HierarchicalTask>> + Send + 'a>>
    {
        Box::pin(async move {
            let response_text = response.as_str().ok_or_else(|| {
                pocketflow_core::error::FlowError::context("Invalid decomposition response format")
            })?;

            let task_id = format!("task_{}_{}", current_depth, uuid::Uuid::new_v4());
            let mut sub_tasks = Vec::new();

            // Simple parsing - in practice this would be more sophisticated
            let mut sub_goal_counter = 1;
            for line in response_text.lines() {
                let line = line.trim();
                if line.starts_with(&format!("{sub_goal_counter}."))
                    || line.starts_with("Sub-goal")
                    || line.starts_with("Subgoal")
                {
                    let description = line.split(':').nth(1).unwrap_or(line).trim().to_string();

                    if !description.is_empty() {
                        let sub_goal = Goal {
                            id: format!("{}_sub_{}", goal.id, sub_goal_counter),
                            description,
                            success_criteria: vec!["sub_goal_completed".to_string()],
                            constraints: goal.constraints.clone(),
                            priority: goal.priority,
                        };

                        // Recursively decompose sub-goals
                        let sub_task = self
                            .decompose_goal(
                                sub_goal,
                                current_depth + 1,
                                Some(task_id.clone()),
                                context,
                            )
                            .await?;

                        sub_tasks.push(sub_task);
                        sub_goal_counter += 1;
                    }
                }
            }

            // If no sub-tasks were parsed, create direct plan steps
            let plan_steps = if sub_tasks.is_empty() {
                self.create_leaf_task_steps(&goal).await?
            } else {
                vec![] // Plan steps are in sub-tasks
            };

            Ok(HierarchicalTask {
                id: task_id,
                goal,
                level: current_depth,
                parent_id,
                sub_tasks,
                plan_steps,
            })
        })
    }

    async fn create_leaf_task_steps(&self, goal: &Goal) -> Result<Vec<PlanStep>> {
        // Create basic steps for leaf tasks
        let steps = vec![
            PlanStep {
                id: format!("{}_init", goal.id),
                description: format!("Initialize: {}", goal.description),
                dependencies: vec![],
                estimated_duration: Duration::from_secs(180),
                required_tools: vec![],
                success_criteria: vec!["initialization_complete".to_string()],
            },
            PlanStep {
                id: format!("{}_execute", goal.id),
                description: format!("Execute: {}", goal.description),
                dependencies: vec![format!("{}_init", goal.id)],
                estimated_duration: Duration::from_secs(300),
                required_tools: vec![],
                success_criteria: goal.success_criteria.clone(),
            },
            PlanStep {
                id: format!("{}_verify", goal.id),
                description: format!("Verify: {}", goal.description),
                dependencies: vec![format!("{}_execute", goal.id)],
                estimated_duration: Duration::from_secs(120),
                required_tools: vec![],
                success_criteria: vec!["verification_complete".to_string()],
            },
        ];

        Ok(steps)
    }

    fn flatten_hierarchical_tasks(&self, root_task: &HierarchicalTask) -> Vec<PlanStep> {
        let mut all_steps = Vec::new();
        collect_steps_recursive(root_task, &mut all_steps);
        all_steps
    }
}

/// Helper function to recursively collect steps from hierarchical tasks
fn collect_steps_recursive(task: &HierarchicalTask, steps: &mut Vec<PlanStep>) {
    // Add this task's direct steps
    steps.extend(task.plan_steps.clone());

    // Recursively collect from sub-tasks
    for sub_task in &task.sub_tasks {
        collect_steps_recursive(sub_task, steps);
    }
}

#[async_trait]
impl<S: FlowState> Node for HierarchicalPlanningNode<S> {
    type State = S;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        let goal = context.get_json::<Goal>("goal")?.ok_or_else(|| {
            pocketflow_core::error::FlowError::context(
                "No goal specified for hierarchical planning",
            )
        })?;

        match self.create_hierarchical_plan(goal, &context).await {
            Ok(execution_plan) => {
                context.set("execution_plan", &execution_plan)?;
                context.set("plan_type", "hierarchical")?;
                context.set("max_hierarchy_depth", self.max_depth)?;

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
impl<S: FlowState> CognitiveNode for HierarchicalPlanningNode<S> {
    type CognitiveOutput = ExecutionPlan;

    async fn think(&self, context: &Context) -> Result<Self::CognitiveOutput> {
        let goal = context.get_json::<Goal>("goal")?.ok_or_else(|| {
            pocketflow_core::error::FlowError::context(
                "No goal available for hierarchical planning",
            )
        })?;

        self.create_hierarchical_plan(goal, context).await
    }
}

#[async_trait]
impl<S: FlowState> PlanningNode for HierarchicalPlanningNode<S> {
    async fn plan(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan> {
        self.create_hierarchical_plan(goal, context).await
    }

    async fn replan(
        &self,
        current_plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ExecutionPlan> {
        // For hierarchical replanning, we recreate the hierarchy
        let updated_goal = Goal {
            id: format!("{}_hierarchical_replan", current_plan.goal.id),
            description: format!("Hierarchical replan: {}", current_plan.goal.description),
            success_criteria: current_plan.goal.success_criteria.clone(),
            constraints: current_plan.goal.constraints.clone(),
            priority: current_plan.goal.priority,
        };

        self.create_hierarchical_plan(updated_goal, context).await
    }

    async fn evaluate_progress(
        &self,
        plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ProgressEvaluation> {
        let completed_steps: Vec<String> = context.get_json("completed_steps")?.unwrap_or_default();

        let total_steps = plan.steps.len();
        let completed_count = completed_steps.len();
        let completion_percentage = if total_steps > 0 {
            (completed_count as f64 / total_steps as f64) * 100.0
        } else {
            0.0
        };

        // Hierarchical-specific recommendations
        let mut recommendations = Vec::new();
        if completion_percentage < 25.0 {
            recommendations.push("Consider simplifying the hierarchy".to_string());
        }
        if total_steps > 20 {
            recommendations
                .push("Plan may be too complex, consider higher-level abstraction".to_string());
        }

        Ok(ProgressEvaluation {
            completion_percentage,
            completed_steps,
            blocked_steps: vec![],      // Could track blocked sub-trees
            issues_encountered: vec![], // Could track coordination issues
            recommendations,
        })
    }
}

/// Builder for HierarchicalPlanningNode
pub struct HierarchicalPlanningNodeBuilder<S: FlowState> {
    name: Option<String>,
    mcp_client: Option<Arc<dyn McpClient>>,
    config: PlanningConfig,
    max_depth: usize,
    decomposition_threshold: usize,
    success_state: Option<S>,
    error_state: Option<S>,
}

impl<S: FlowState> HierarchicalPlanningNodeBuilder<S> {
    pub fn new() -> Self {
        Self {
            name: None,
            mcp_client: None,
            config: PlanningConfig {
                strategy: PlanningStrategy::Hierarchical,
                max_plan_depth: 4,
                ..PlanningConfig::default()
            },
            max_depth: 4,
            decomposition_threshold: 5,
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

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_decomposition_threshold(mut self, threshold: usize) -> Self {
        self.decomposition_threshold = threshold;
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

    pub fn build(self) -> Result<HierarchicalPlanningNode<S>> {
        let name = self
            .name
            .unwrap_or_else(|| "hierarchical_planner".to_string());
        let mcp_client = self.mcp_client.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("MCP client is required")
        })?;
        let success_state = self.success_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Success state is required")
        })?;
        let error_state = self.error_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Error state is required")
        })?;

        Ok(HierarchicalPlanningNode::new(
            name,
            mcp_client,
            self.config,
            self.max_depth,
            self.decomposition_threshold,
            success_state,
            error_state,
        ))
    }
}

impl<S: FlowState> Default for HierarchicalPlanningNodeBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
