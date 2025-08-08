//! Core traits for cognitive extensions.
//!
//! This module defines the extension traits that add cognitive capabilities
//! to existing PocketFlow nodes without modifying the core Node trait.

use std::fmt::Debug;

use async_trait::async_trait;
use pocketflow_core::{context::Context, node::Node};
use serde::{Deserialize, Serialize};

use crate::Result;

/// Core trait for nodes with cognitive capabilities.
///
/// This trait extends the basic Node functionality with thinking and reasoning
/// capabilities while maintaining compatibility with the existing Node trait.
#[async_trait]
pub trait CognitiveNode: Node + Send + Sync {
    /// The output type for cognitive operations
    type CognitiveOutput: Send + Sync + Debug;

    /// Perform cognitive processing on the given context.
    ///
    /// This is the core method that enables thinking, reasoning, and other
    /// cognitive operations without modifying the original Node::execute method.
    async fn think(&self, context: &Context) -> Result<Self::CognitiveOutput>;

    /// Optional: Prepare cognitive resources before execution
    async fn prepare_cognitive(&self, _context: &Context) -> Result<()> {
        Ok(())
    }

    /// Optional: Cleanup cognitive resources after execution  
    async fn cleanup_cognitive(&self, _context: &Context) -> Result<()> {
        Ok(())
    }
}

/// Trait for nodes that can perform multi-step reasoning.
///
/// This trait enables chain-of-thought reasoning, reflection, and other
/// advanced thinking patterns.
#[async_trait]
pub trait ThinkingNode: CognitiveNode {
    /// Perform step-by-step reasoning on a problem
    async fn reason(&self, context: &Context) -> Result<ReasoningChain>;

    /// Reflect on previous reasoning or execution results
    async fn reflect(
        &self,
        context: &Context,
        previous_result: &ExecutionResult,
    ) -> Result<Reflection>;

    /// Generate explanations for decisions made
    async fn explain(&self, context: &Context, decision: &Decision) -> Result<Explanation>;
}

/// Trait for nodes that can perform goal-oriented planning.
///
/// This trait enables decomposition of high-level goals into actionable plans
/// and adaptive replanning based on execution feedback.
#[async_trait]
pub trait PlanningNode: CognitiveNode {
    /// Create an execution plan for achieving the given goal
    async fn plan(&self, goal: Goal, context: &Context) -> Result<ExecutionPlan>;

    /// Adapt or recreate a plan based on current context and feedback
    async fn replan(
        &self,
        current_plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ExecutionPlan>;

    /// Evaluate the progress of an execution plan
    async fn evaluate_progress(
        &self,
        plan: &ExecutionPlan,
        context: &Context,
    ) -> Result<ProgressEvaluation>;
}

/// Trait for nodes that can learn and adapt from experience.
#[async_trait]
pub trait LearningNode: CognitiveNode {
    /// Update internal models based on execution feedback
    async fn learn(&mut self, experience: &Experience) -> Result<()>;

    /// Retrieve similar past experiences for current context
    async fn recall_experience(&self, context: &Context) -> Result<Vec<Experience>>;
}

/// Represents a chain of reasoning steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningChain {
    pub steps: Vec<ReasoningStep>,
    pub conclusion: String,
    pub confidence: f64,
    pub alternatives_considered: Vec<Alternative>,
}

/// Individual step in a reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_number: usize,
    pub thought: String,
    pub evidence: Vec<String>,
    pub inference: String,
    pub confidence: f64,
}

/// Result of a reflection process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub insights: Vec<String>,
    pub identified_errors: Vec<String>,
    pub improvement_suggestions: Vec<String>,
    pub confidence_in_result: f64,
}

/// Explanation for a decision or reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub reasoning: String,
    pub key_factors: Vec<String>,
    pub alternatives_rejected: Vec<String>,
    pub confidence_level: f64,
}

/// Represents a high-level goal to be achieved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub success_criteria: Vec<String>,
    pub constraints: Vec<String>,
    pub priority: u8, // 1-10 scale
}

/// Detailed execution plan for achieving a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub id: String,
    pub goal: Goal,
    pub steps: Vec<PlanStep>,
    pub estimated_duration: std::time::Duration,
    pub required_resources: Vec<String>,
    pub risk_factors: Vec<String>,
}

/// Individual step in an execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub estimated_duration: std::time::Duration,
    pub required_tools: Vec<String>,
    /// Rich success criteria.
    /// - String: substring must appear in textual output
    /// - Object {"regex": "pattern"}: regex must match textual output
    /// - Object {"json_pointer": "/path", "equals": json}: JSON output at pointer equals value
    /// - Object {"json_pointer": "/path", "exists": true}: pointer exists
    /// - Object {"json_pointer": "/path", "contains": "substr"}: when pointer resolves to a string, it must contain the substring;
    ///   when it resolves to an array of strings, at least one element must contain the substring
    pub success_criteria: Vec<serde_json::Value>,
    /// Optional per-step overrides for executor behavior
    #[serde(default)]
    pub enforce_success_criteria: Option<bool>,
    #[serde(default)]
    pub max_retries: Option<usize>,
    #[serde(default)]
    pub initial_backoff_ms: Option<u64>,
    #[serde(default)]
    pub stop_on_error: Option<bool>,
}

/// Progress evaluation for an execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvaluation {
    pub completion_percentage: f64,
    pub completed_steps: Vec<String>,
    pub blocked_steps: Vec<String>,
    pub issues_encountered: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Execution result that can be reflected upon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub duration: std::time::Duration,
    pub errors: Vec<String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Decision point that can be explained
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub decision_point: String,
    pub chosen_option: String,
    pub available_options: Vec<String>,
    pub decision_criteria: Vec<String>,
}

/// Alternative option considered during reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub confidence: f64,
}

/// Learning experience for adaptive nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub context_snapshot: serde_json::Value,
    pub action_taken: String,
    pub outcome: ExecutionResult,
    pub feedback: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Extension trait for adding cognitive capabilities to any Node
pub trait CognitiveNodeExt: Node {
    /// Wrap this node with cognitive capabilities
    fn with_cognitive<C: CognitiveNode<State = Self::State>>(
        self,
        cognitive_impl: C,
    ) -> CognitiveWrapper<Self, C>
    where
        Self: Sized,
    {
        CognitiveWrapper::new(self, cognitive_impl)
    }
}

// Blanket implementation for all nodes
impl<T: Node> CognitiveNodeExt for T {}

/// Wrapper that adds cognitive capabilities to any existing node
#[derive(Debug)]
pub struct CognitiveWrapper<N, C>
where
    N: Node,
    C: CognitiveNode<State = N::State>,
{
    inner_node: N,
    cognitive_impl: C,
}

impl<N, C> CognitiveWrapper<N, C>
where
    N: Node,
    C: CognitiveNode<State = N::State>,
{
    pub fn new(inner_node: N, cognitive_impl: C) -> Self {
        Self {
            inner_node,
            cognitive_impl,
        }
    }
}

#[async_trait]
impl<N, C> Node for CognitiveWrapper<N, C>
where
    N: Node,
    C: CognitiveNode<State = N::State>,
{
    type State = N::State;

    async fn execute(&self, context: Context) -> crate::Result<(Context, Self::State)> {
        // First perform cognitive processing
        self.cognitive_impl.prepare_cognitive(&context).await?;
        let _cognitive_output = self.cognitive_impl.think(&context).await?;

        // Then execute the original node
        let result = self.inner_node.execute(context).await?;

        // Finally cleanup cognitive resources
        self.cognitive_impl.cleanup_cognitive(&result.0).await?;

        Ok(result)
    }

    fn name(&self) -> String {
        format!("Cognitive({})", self.inner_node.name())
    }
}
