//! Utility functions and helpers for cognitive operations.

use std::time::Duration;

use pocketflow_core::error::FlowError;

use crate::traits::{ExecutionPlan, Goal, PlanStep};

/// Helper functions for creating common cognitive structures
pub mod builders {
    use super::*;

    /// Create a simple goal with basic parameters
    pub fn simple_goal(id: impl Into<String>, description: impl Into<String>) -> Goal {
        Goal {
            id: id.into(),
            description: description.into(),
            success_criteria: vec!["Task completed successfully".to_string()],
            constraints: vec![],
            priority: 5,
        }
    }

    /// Create a goal with detailed parameters
    pub fn detailed_goal(
        id: impl Into<String>,
        description: impl Into<String>,
        success_criteria: Vec<String>,
        constraints: Vec<String>,
        priority: u8,
    ) -> Goal {
        Goal {
            id: id.into(),
            description: description.into(),
            success_criteria,
            constraints,
            priority,
        }
    }

    /// Create a simple plan step
    pub fn simple_step(id: impl Into<String>, description: impl Into<String>) -> PlanStep {
        PlanStep {
            id: id.into(),
            description: description.into(),
            dependencies: vec![],
            estimated_duration: Duration::from_secs(300), // 5 minutes default
            required_tools: vec![],
            success_criteria: vec![serde_json::json!("Step completed")],
            enforce_success_criteria: None,
            max_retries: None,
            initial_backoff_ms: None,
            stop_on_error: None,
        }
    }

    /// Create a detailed plan step
    pub fn detailed_step(
        id: impl Into<String>,
        description: impl Into<String>,
        dependencies: Vec<String>,
        duration: Duration,
        tools: Vec<String>,
        success_criteria: Vec<serde_json::Value>,
    ) -> PlanStep {
        PlanStep {
            id: id.into(),
            description: description.into(),
            dependencies,
            estimated_duration: duration,
            required_tools: tools,
            success_criteria,
            enforce_success_criteria: None,
            max_retries: None,
            initial_backoff_ms: None,
            stop_on_error: None,
        }
    }

    /// Create an execution plan from a goal and steps
    pub fn execution_plan(goal: Goal, steps: Vec<PlanStep>) -> ExecutionPlan {
        let total_duration: Duration = steps.iter().map(|step| step.estimated_duration).sum();

        let required_resources: Vec<String> = steps
            .iter()
            .flat_map(|step| step.required_tools.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        ExecutionPlan {
            id: format!("plan_{}", uuid::Uuid::new_v4()),
            goal,
            steps,
            estimated_duration: total_duration,
            required_resources,
            risk_factors: vec![],
        }
    }
}

/// Validation utilities for cognitive structures
pub mod validation {
    use super::*;

    /// Validates a goal for common issues
    pub fn validate_goal(goal: &Goal) -> Result<(), FlowError> {
        if goal.id.trim().is_empty() {
            return Err(crate::error::CognitiveError::InvalidConfig {
                message: "Goal ID cannot be empty".to_string(),
            }
            .into());
        }

        if goal.description.trim().is_empty() {
            return Err(crate::error::CognitiveError::InvalidConfig {
                message: "Goal description cannot be empty".to_string(),
            }
            .into());
        }

        if goal.priority < 1 || goal.priority > 10 {
            return Err(crate::error::CognitiveError::InvalidConfig {
                message: "Goal priority must be between 1 and 10".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Validate that an execution plan is well-formed
    pub fn validate_execution_plan(plan: &ExecutionPlan) -> Result<(), FlowError> {
        validate_goal(&plan.goal)?;

        if plan.steps.is_empty() {
            return Err(crate::error::CognitiveError::InvalidConfig {
                message: "Execution plan must have at least one step".to_string(),
            }
            .into());
        }

        // Check for circular dependencies
        for step in &plan.steps {
            if step.dependencies.contains(&step.id) {
                return Err(crate::error::CognitiveError::InvalidConfig {
                    message: format!("Step {} has circular dependency on itself", step.id),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Validate that plan steps have valid dependencies
    pub fn validate_step_dependencies(steps: &[PlanStep]) -> Result<(), FlowError> {
        let step_ids: std::collections::HashSet<_> = steps.iter().map(|s| &s.id).collect();

        for step in steps {
            for dep in &step.dependencies {
                if !step_ids.contains(dep) {
                    return Err(crate::error::CognitiveError::InvalidConfig {
                        message: format!("Step {} depends on non-existent step {}", step.id, dep),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }
}

/// Analysis utilities for cognitive operations
pub mod analysis {
    use super::*;
    use crate::memory::Episode;

    /// Analyze episodes to find success patterns
    pub fn analyze_success_patterns(episodes: &[Episode]) -> Vec<String> {
        let successful_episodes: Vec<_> = episodes.iter().filter(|ep| ep.success).collect();

        if successful_episodes.is_empty() {
            return vec![];
        }

        // Extract common patterns from successful episodes
        let mut patterns = Vec::new();

        // Pattern 1: Common action types
        let action_frequency = count_action_frequencies(&successful_episodes);
        for (action, count) in action_frequency {
            if count > successful_episodes.len() / 2 {
                patterns.push(format!(
                    "Successful pattern: {} (used in {}% of successes)",
                    action,
                    (count * 100) / successful_episodes.len()
                ));
            }
        }

        patterns
    }

    fn count_action_frequencies(episodes: &[&Episode]) -> std::collections::HashMap<String, usize> {
        let mut frequencies = std::collections::HashMap::new();

        for episode in episodes {
            // Simple word extraction from action_taken
            let words: Vec<&str> = episode.action_taken.split_whitespace().collect();

            for word in words {
                let word = word.to_lowercase();
                if word.len() > 3 {
                    // Filter out short words
                    *frequencies.entry(word).or_insert(0) += 1;
                }
            }
        }

        frequencies
    }

    /// Calculate confidence based on past success rate
    pub fn calculate_confidence(similar_episodes: &[Episode], base_confidence: f64) -> f64 {
        if similar_episodes.is_empty() {
            return base_confidence;
        }

        let success_rate = similar_episodes.iter().filter(|ep| ep.success).count() as f64
            / similar_episodes.len() as f64;

        // Weighted average of base confidence and historical success rate
        (base_confidence + success_rate) / 2.0
    }

    /// Estimate duration based on similar past episodes
    pub fn estimate_duration(similar_episodes: &[Episode]) -> Duration {
        if similar_episodes.is_empty() {
            return Duration::from_secs(300); // Default 5 minutes
        }

        let total_duration: Duration = similar_episodes.iter().map(|ep| ep.duration).sum();

        total_duration / similar_episodes.len() as u32
    }
}
