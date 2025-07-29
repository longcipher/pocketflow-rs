//! Planning and goal-oriented reasoning node implementations.
//!
//! This module provides concrete implementations of planning capabilities
//! including goal decomposition, execution planning, and adaptive replanning.

use serde::{Deserialize, Serialize};

pub mod adaptive;
pub mod goal_oriented;
pub mod hierarchical;

// Re-export the planning node implementations
pub use adaptive::{AdaptivePlanningNode, AdaptivePlanningNodeBuilder};
pub use goal_oriented::{GoalOrientedPlanningNode, GoalOrientedPlanningNodeBuilder};
pub use hierarchical::{
    HierarchicalPlanningNode, HierarchicalPlanningNodeBuilder, HierarchicalTask,
};

/// Strategy for planning approach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanningStrategy {
    /// Linear step-by-step planning
    Sequential,
    /// Hierarchical task decomposition
    Hierarchical,
    /// Parallel execution planning
    Parallel,
    /// Adaptive planning with feedback loops
    Adaptive,
    /// Goal-oriented backward planning
    BackwardChaining,
}

/// Configuration for planning operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningConfig {
    pub strategy: PlanningStrategy,
    pub max_plan_depth: usize,
    pub max_steps_per_plan: usize,
    pub enable_risk_assessment: bool,
    pub enable_resource_estimation: bool,
    pub replanning_threshold: f64,
}

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            strategy: PlanningStrategy::Sequential,
            max_plan_depth: 5,
            max_steps_per_plan: 20,
            enable_risk_assessment: true,
            enable_resource_estimation: true,
            replanning_threshold: 0.3,
        }
    }
}
