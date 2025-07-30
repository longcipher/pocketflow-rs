use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use pocketflow_core::prelude::{Context, FlowError, FlowState, Node};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::{
    agent_node::{AgentNode, AgentRegistry},
    agent_types::{AgentResult, AgentStep},
    error::{AgentError, Result},
};

/// Multi-agent coordination strategies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    /// Execute agents sequentially
    Sequential,
    /// Execute agents in parallel
    Parallel,
    /// Hierarchical delegation from coordinator to specialists
    Hierarchical,
    /// Democratic voting on best result
    Voting,
    /// Round-robin execution
    RoundRobin,
    /// Custom strategy
    Custom(String),
}

/// Multi-agent execution states
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MultiAgentState {
    /// Ready to start
    Ready,
    /// Planning execution
    Planning,
    /// Agents are executing
    Executing {
        active_agents: Vec<String>,
        completed_agents: Vec<String>,
    },
    /// Waiting for agent completion
    Waiting { agent_name: String },
    /// Coordinating results
    Coordinating,
    /// All agents completed successfully
    Completed { final_result: String },
    /// Execution failed
    Failed { error: String },
}

impl FlowState for MultiAgentState {
    fn is_terminal(&self) -> bool {
        matches!(
            self,
            MultiAgentState::Completed { .. } | MultiAgentState::Failed { .. }
        )
    }
}

/// Multi-agent node for coordinating multiple AI agents
#[derive(Debug)]
pub struct MultiAgentNode {
    name: String,
    coordinator: Option<Arc<AgentNode>>,
    agents: HashMap<String, Arc<AgentNode>>,
    strategy: CoordinationStrategy,
    agent_registry: Arc<AgentRegistry>,
    execution_plan: Option<ExecutionPlan>,
    max_parallel_agents: usize,
}

impl MultiAgentNode {
    pub fn new(
        name: impl Into<String>,
        strategy: CoordinationStrategy,
        agent_registry: Arc<AgentRegistry>,
    ) -> Self {
        Self {
            name: name.into(),
            coordinator: None,
            agents: HashMap::new(),
            strategy,
            agent_registry,
            execution_plan: None,
            max_parallel_agents: 3,
        }
    }

    pub fn with_coordinator(mut self, coordinator: Arc<AgentNode>) -> Self {
        self.coordinator = Some(coordinator);
        self
    }

    pub fn add_agent(mut self, name: impl Into<String>, agent: Arc<AgentNode>) -> Self {
        self.agents.insert(name.into(), agent);
        self
    }

    pub fn with_strategy(mut self, strategy: CoordinationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_max_parallel_agents(mut self, max: usize) -> Self {
        self.max_parallel_agents = max;
        self
    }

    pub fn with_execution_plan(mut self, plan: ExecutionPlan) -> Self {
        self.execution_plan = Some(plan);
        self
    }

    /// Execute multiple agents with the given strategy
    pub async fn execute_multi_agent(&self, task: &str) -> Result<MultiAgentResult> {
        let start_time = std::time::Instant::now();
        info!(
            "Starting multi-agent execution with strategy {:?} for task: {}",
            self.strategy, task
        );

        match self.strategy {
            CoordinationStrategy::Sequential => self.execute_sequential(task, start_time).await,
            CoordinationStrategy::Parallel => self.execute_parallel(task, start_time).await,
            CoordinationStrategy::Hierarchical => self.execute_hierarchical(task, start_time).await,
            CoordinationStrategy::Voting => self.execute_voting(task, start_time).await,
            CoordinationStrategy::RoundRobin => self.execute_round_robin(task, start_time).await,
            CoordinationStrategy::Custom(_) => Err(AgentError::coordination(
                "Custom strategies not yet implemented",
            )),
        }
    }

    async fn execute_sequential(
        &self,
        task: &str,
        start_time: std::time::Instant,
    ) -> Result<MultiAgentResult> {
        let mut results = HashMap::new();
        let mut all_steps = Vec::new();
        let mut current_input = task.to_string();

        // Sort agents by priority
        let mut sorted_agents: Vec<_> = self.agents.iter().collect();
        sorted_agents.sort_by(|a, b| b.1.config.priority.cmp(&a.1.config.priority));

        for (agent_name, agent) in sorted_agents {
            info!("Executing agent '{}' sequentially", agent_name);

            match agent.execute_task(&current_input).await {
                Ok(result) => {
                    if result.success {
                        if let Some(answer) = &result.final_answer {
                            current_input = answer.clone(); // Chain outputs
                        }
                        results.insert(agent_name.clone(), result.clone());
                        all_steps.extend(result.steps);
                    } else {
                        let error = result.error.unwrap_or_else(|| "Agent failed".to_string());
                        error!("Agent '{}' failed: {}", agent_name, error);
                        return Ok(MultiAgentResult::failure(
                            format!("Agent '{agent_name}' failed: {error}"),
                            results,
                            all_steps,
                            start_time.elapsed(),
                        ));
                    }
                }
                Err(e) => {
                    error!("Agent '{}' error: {}", agent_name, e);
                    return Ok(MultiAgentResult::failure(
                        format!("Agent '{agent_name}' error: {e}"),
                        results,
                        all_steps,
                        start_time.elapsed(),
                    ));
                }
            }
        }

        // Use the last agent's result as final result
        let final_result = results
            .values()
            .last()
            .and_then(|r| r.final_answer.clone())
            .unwrap_or_else(|| "No final result".to_string());

        Ok(MultiAgentResult::success(
            final_result,
            results,
            all_steps,
            start_time.elapsed(),
        ))
    }

    async fn execute_parallel(
        &self,
        task: &str,
        start_time: std::time::Instant,
    ) -> Result<MultiAgentResult> {
        info!("Executing {} agents in parallel", self.agents.len());

        // Execute all agents concurrently
        let mut handles = Vec::new();
        for (agent_name, agent) in &self.agents {
            let agent_clone = agent.clone();
            let task_clone = task.to_string();
            let name_clone = agent_name.clone();

            let handle =
                tokio::spawn(
                    async move { (name_clone, agent_clone.execute_task(&task_clone).await) },
                );
            handles.push(handle);
        }

        // Collect results
        let mut results = HashMap::new();
        let mut all_steps = Vec::new();
        let mut has_success = false;
        let mut final_result = String::new();

        for handle in handles {
            match handle.await {
                Ok((agent_name, agent_result)) => {
                    match agent_result {
                        Ok(result) => {
                            if result.success {
                                has_success = true;
                                if let Some(answer) = &result.final_answer {
                                    final_result = answer.clone(); // Use last successful result
                                }
                            }
                            results.insert(agent_name.clone(), result.clone());
                            all_steps.extend(result.steps);
                        }
                        Err(e) => {
                            warn!("Agent '{}' failed: {}", agent_name, e);
                            results.insert(
                                agent_name,
                                AgentResult::failure(
                                    e.to_string(),
                                    Vec::new(),
                                    std::time::Duration::default(),
                                ),
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Agent task join error: {}", e);
                }
            }
        }

        if has_success {
            Ok(MultiAgentResult::success(
                final_result,
                results,
                all_steps,
                start_time.elapsed(),
            ))
        } else {
            Ok(MultiAgentResult::failure(
                "All agents failed".to_string(),
                results,
                all_steps,
                start_time.elapsed(),
            ))
        }
    }

    async fn execute_hierarchical(
        &self,
        task: &str,
        start_time: std::time::Instant,
    ) -> Result<MultiAgentResult> {
        if let Some(coordinator) = &self.coordinator {
            info!("Executing hierarchical coordination with coordinator");

            // Register all agents with the coordinator's registry
            for agent in self.agents.values() {
                self.agent_registry
                    .register(agent.config.name.clone(), agent.clone())
                    .await;
            }

            // Execute coordinator
            match coordinator.execute_task(task).await {
                Ok(result) => {
                    if result.success {
                        Ok(MultiAgentResult::success(
                            result.final_answer.clone().unwrap_or_default(),
                            [(coordinator.config.name.clone(), result.clone())].into(),
                            result.steps,
                            start_time.elapsed(),
                        ))
                    } else {
                        Ok(MultiAgentResult::failure(
                            result
                                .error
                                .clone()
                                .unwrap_or_else(|| "Coordinator failed".to_string()),
                            [(coordinator.config.name.clone(), result.clone())].into(),
                            result.steps,
                            start_time.elapsed(),
                        ))
                    }
                }
                Err(e) => Ok(MultiAgentResult::failure(
                    e.to_string(),
                    HashMap::new(),
                    Vec::new(),
                    start_time.elapsed(),
                )),
            }
        } else {
            Err(AgentError::configuration(
                "Hierarchical strategy requires a coordinator agent",
            ))
        }
    }

    async fn execute_voting(
        &self,
        task: &str,
        start_time: std::time::Instant,
    ) -> Result<MultiAgentResult> {
        info!(
            "Executing voting strategy with {} agents",
            self.agents.len()
        );

        // Execute all agents in parallel
        let parallel_result = self.execute_parallel(task, start_time).await?;

        if !parallel_result.success {
            return Ok(parallel_result);
        }

        // Collect all successful results for voting
        let successful_results: Vec<_> = parallel_result
            .agent_results
            .values()
            .filter(|r| r.success && r.final_answer.is_some())
            .collect();

        if successful_results.is_empty() {
            return Ok(MultiAgentResult::failure(
                "No successful results to vote on".to_string(),
                parallel_result.agent_results,
                parallel_result.all_steps,
                parallel_result.total_duration,
            ));
        }

        // Simple majority voting (could be enhanced with ranking, scoring, etc.)
        let mut vote_counts: HashMap<String, usize> = HashMap::new();
        for result in &successful_results {
            if let Some(answer) = &result.final_answer {
                *vote_counts.entry(answer.clone()).or_insert(0) += 1;
            }
        }

        // Find the answer with most votes
        let winning_answer = vote_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(answer, _)| answer)
            .unwrap_or_else(|| "No consensus reached".to_string());

        Ok(MultiAgentResult::success(
            winning_answer,
            parallel_result.agent_results,
            parallel_result.all_steps,
            parallel_result.total_duration,
        ))
    }

    async fn execute_round_robin(
        &self,
        task: &str,
        start_time: std::time::Instant,
    ) -> Result<MultiAgentResult> {
        info!("Executing round-robin strategy");

        let mut results = HashMap::new();
        let mut all_steps = Vec::new();
        let mut current_task = task.to_string();
        let mut round = 1;
        const MAX_ROUNDS: usize = 3;

        while round <= MAX_ROUNDS {
            info!("Round-robin round {}", round);

            for (agent_name, agent) in &self.agents {
                info!("Agent '{}' executing in round {}", agent_name, round);

                match agent.execute_task(&current_task).await {
                    Ok(result) => {
                        results.insert(format!("{agent_name}_{round}"), result.clone());
                        all_steps.extend(result.steps);

                        if result.success
                            && let Some(answer) = &result.final_answer
                        {
                            // Check if this looks like a final answer
                            if answer.to_lowercase().contains("final")
                                || answer.to_lowercase().contains("complete")
                                || round == MAX_ROUNDS
                            {
                                return Ok(MultiAgentResult::success(
                                    answer.clone(),
                                    results,
                                    all_steps,
                                    start_time.elapsed(),
                                ));
                            }
                            // Use this as input for next round
                            current_task = format!(
                                "Previous result: {answer}\n\nRefine or improve this result."
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Agent '{}' failed in round {}: {}", agent_name, round, e);
                    }
                }
            }

            round += 1;
        }

        // Return best result from final round
        let final_result = results
            .values()
            .filter(|r| r.success)
            .last()
            .and_then(|r| r.final_answer.clone())
            .unwrap_or_else(|| "Round-robin completed without final answer".to_string());

        Ok(MultiAgentResult::success(
            final_result,
            results,
            all_steps,
            start_time.elapsed(),
        ))
    }
}

#[async_trait]
impl Node for MultiAgentNode {
    type State = MultiAgentState;

    async fn execute(
        &self,
        context: Context,
    ) -> std::result::Result<(Context, Self::State), FlowError> {
        // Extract task from context
        let task: String = context
            .get_json("task")?
            .and_then(|v: serde_json::Value| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| FlowError::context("No task provided in context"))?;

        // Execute multi-agent workflow
        match self.execute_multi_agent(&task).await {
            Ok(result) => {
                let mut new_context = context;

                // Store result in context
                new_context.set("multi_agent_result", serde_json::to_value(&result)?)?;
                new_context.set(
                    "agent_results",
                    serde_json::to_value(&result.agent_results)?,
                )?;
                new_context.set("all_steps", serde_json::to_value(&result.all_steps)?)?;

                if result.success {
                    new_context.set("final_answer", &result.final_result)?;
                    Ok((
                        new_context,
                        MultiAgentState::Completed {
                            final_result: result.final_result,
                        },
                    ))
                } else {
                    let error_msg = result.error.clone().unwrap_or_default();
                    new_context.set("error", &error_msg)?;
                    Ok((
                        new_context,
                        MultiAgentState::Failed {
                            error: result
                                .error
                                .unwrap_or_else(|| "Unknown multi-agent error".to_string()),
                        },
                    ))
                }
            }
            Err(e) => {
                let mut new_context = context;
                new_context.set("error", e.to_string())?;
                Ok((
                    new_context,
                    MultiAgentState::Failed {
                        error: e.to_string(),
                    },
                ))
            }
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Multi-agent execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentResult {
    pub success: bool,
    pub final_result: String,
    pub agent_results: HashMap<String, AgentResult>,
    pub all_steps: Vec<AgentStep>,
    pub total_duration: std::time::Duration,
    pub error: Option<String>,
    pub coordination_metadata: HashMap<String, serde_json::Value>,
}

impl MultiAgentResult {
    pub fn success(
        final_result: String,
        agent_results: HashMap<String, AgentResult>,
        all_steps: Vec<AgentStep>,
        duration: std::time::Duration,
    ) -> Self {
        Self {
            success: true,
            final_result,
            agent_results,
            all_steps,
            total_duration: duration,
            error: None,
            coordination_metadata: HashMap::new(),
        }
    }

    pub fn failure(
        error: String,
        agent_results: HashMap<String, AgentResult>,
        all_steps: Vec<AgentStep>,
        duration: std::time::Duration,
    ) -> Self {
        Self {
            success: false,
            final_result: String::new(),
            agent_results,
            all_steps,
            total_duration: duration,
            error: Some(error),
            coordination_metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.coordination_metadata.insert(key.into(), value);
        self
    }
}

/// Execution plan for complex multi-agent workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub phases: Vec<ExecutionPhase>,
    pub dependencies: HashMap<String, Vec<String>>,
    pub timeout: Option<std::time::Duration>,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            phases: Vec::new(),
            dependencies: HashMap::new(),
            timeout: None,
        }
    }

    pub fn add_phase(mut self, phase: ExecutionPhase) -> Self {
        self.phases.push(phase);
        self
    }

    pub fn add_dependency(mut self, agent: impl Into<String>, depends_on: Vec<String>) -> Self {
        self.dependencies.insert(agent.into(), depends_on);
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution phase in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    pub name: String,
    pub agents: Vec<String>,
    pub strategy: CoordinationStrategy,
    pub timeout: Option<std::time::Duration>,
}

impl ExecutionPhase {
    pub fn new(name: impl Into<String>, agents: Vec<String>) -> Self {
        Self {
            name: name.into(),
            agents,
            strategy: CoordinationStrategy::Parallel,
            timeout: None,
        }
    }

    pub fn with_strategy(mut self, strategy: CoordinationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Builder for MultiAgentNode
pub struct MultiAgentNodeBuilder {
    name: String,
    coordinator: Option<Arc<AgentNode>>,
    agents: HashMap<String, Arc<AgentNode>>,
    strategy: CoordinationStrategy,
    max_parallel_agents: usize,
    execution_plan: Option<ExecutionPlan>,
}

impl MultiAgentNodeBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            coordinator: None,
            agents: HashMap::new(),
            strategy: CoordinationStrategy::Sequential,
            max_parallel_agents: 3,
            execution_plan: None,
        }
    }

    pub fn with_coordinator(mut self, coordinator: Arc<AgentNode>) -> Self {
        self.coordinator = Some(coordinator);
        self
    }

    pub fn add_agent(mut self, name: impl Into<String>, agent: Arc<AgentNode>) -> Self {
        self.agents.insert(name.into(), agent);
        self
    }

    pub fn with_strategy(mut self, strategy: CoordinationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_max_parallel_agents(mut self, max: usize) -> Self {
        self.max_parallel_agents = max;
        self
    }

    pub fn with_execution_plan(mut self, plan: ExecutionPlan) -> Self {
        self.execution_plan = Some(plan);
        self
    }

    pub async fn build(self) -> Result<MultiAgentNode> {
        let agent_registry = Arc::new(AgentRegistry::new());

        // Register all agents
        for agent in self.agents.values() {
            agent_registry
                .register(agent.config.name.clone(), agent.clone())
                .await;
        }

        let mut node = MultiAgentNode::new(self.name, self.strategy, agent_registry);

        if let Some(coordinator) = self.coordinator {
            node = node.with_coordinator(coordinator);
        }

        for (name, agent) in self.agents {
            node = node.add_agent(name, agent);
        }

        node = node.with_max_parallel_agents(self.max_parallel_agents);

        if let Some(plan) = self.execution_plan {
            node = node.with_execution_plan(plan);
        }

        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::AgentNodeBuilder;

    #[tokio::test]
    async fn test_multi_agent_node_creation() {
        let agent1 = Arc::new(
            AgentNodeBuilder::new("agent1", "First agent")
                .with_openai_model("gpt-4o-mini")
                .build()
                .await
                .unwrap(),
        );

        let agent2 = Arc::new(
            AgentNodeBuilder::new("agent2", "Second agent")
                .with_openai_model("gpt-4o-mini")
                .build()
                .await
                .unwrap(),
        );

        let multi_agent = MultiAgentNodeBuilder::new("team")
            .add_agent("agent1", agent1)
            .add_agent("agent2", agent2)
            .with_strategy(CoordinationStrategy::Parallel)
            .build()
            .await
            .unwrap();

        assert_eq!(multi_agent.name(), "team");
        assert_eq!(multi_agent.agents.len(), 2);
    }

    #[test]
    fn test_execution_plan() {
        let plan = ExecutionPlan::new()
            .add_phase(
                ExecutionPhase::new(
                    "research",
                    vec!["researcher1".to_string(), "researcher2".to_string()],
                )
                .with_strategy(CoordinationStrategy::Parallel),
            )
            .add_phase(
                ExecutionPhase::new("analysis", vec!["analyst".to_string()])
                    .with_strategy(CoordinationStrategy::Sequential),
            )
            .add_dependency(
                "analyst",
                vec!["researcher1".to_string(), "researcher2".to_string()],
            )
            .with_timeout(std::time::Duration::from_secs(300));

        assert_eq!(plan.phases.len(), 2);
        assert!(plan.dependencies.contains_key("analyst"));
        assert!(plan.timeout.is_some());
    }

    #[test]
    fn test_multi_agent_state_transitions() {
        use MultiAgentState::*;

        assert!(Ready.can_transition_to(&Planning));
        assert!(Planning.can_transition_to(&Executing {
            active_agents: vec!["agent1".to_string()],
            completed_agents: vec![]
        }));
        assert!(
            !Completed {
                final_result: "done".to_string()
            }
            .can_transition_to(&Planning)
        );
    }
}
