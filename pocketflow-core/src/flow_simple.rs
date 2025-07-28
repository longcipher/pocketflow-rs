//! Simple flow orchestration without dptree complexity.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    context::Context,
    error::{FlowError, Result},
    node::Node,
    state::FlowState,
};

/// Workflow execution result.
#[derive(Debug, Clone)]
pub struct FlowResult<S: FlowState> {
    /// Final execution state.
    pub final_state: S,
    /// Final context after execution.
    pub context: Context,
    /// Total execution time.
    pub duration: Duration,
    /// Number of steps executed.
    pub steps: usize,
    /// Whether the flow completed successfully.
    pub success: bool,
    /// Any error that occurred.
    pub error: Option<String>,
}

/// Simple workflow execution engine.
pub struct SimpleFlow<S: FlowState> {
    nodes: HashMap<S, Arc<dyn Node<State = S>>>,
    initial_state: S,
    name: String,
}

impl<S: FlowState> SimpleFlow<S> {
    /// Create a new flow builder.
    pub fn builder() -> SimpleFlowBuilder<S> {
        SimpleFlowBuilder::new()
    }

    /// Execute the workflow.
    pub async fn execute(&self, mut context: Context) -> Result<FlowResult<S>> {
        let start_time = Instant::now();
        let mut current_state = self.initial_state.clone();
        let mut steps = 0;

        loop {
            steps += 1;

            // Prevent infinite loops
            if steps > 1000 {
                return Err(FlowError::execution("Flow exceeded maximum steps (1000)"));
            }

            // Check if we've reached a terminal state
            if current_state.is_terminal() {
                return Ok(FlowResult {
                    final_state: current_state,
                    context,
                    duration: start_time.elapsed(),
                    steps,
                    success: true,
                    error: None,
                });
            }

            // Find the node for the current state
            let node = self.nodes.get(&current_state).ok_or_else(|| {
                FlowError::execution(format!("No node found for state: {current_state:?}"))
            })?;

            // Execute the node
            let node_result = node.execute(context).await;
            match node_result {
                Ok((new_context, new_state)) => {
                    context = new_context;
                    current_state = new_state;
                }
                Err(error) => {
                    // We can't use context here since it was moved to node.execute
                    // Create a new empty context for the error result
                    return Ok(FlowResult {
                        final_state: current_state,
                        context: Context::new(),
                        duration: start_time.elapsed(),
                        steps,
                        success: false,
                        error: Some(error.to_string()),
                    });
                }
            }
        }
    }

    /// Get the flow name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Builder for SimpleFlow.
pub struct SimpleFlowBuilder<S: FlowState> {
    nodes: HashMap<S, Arc<dyn Node<State = S>>>,
    initial_state: Option<S>,
    name: String,
}

impl<S: FlowState> SimpleFlowBuilder<S> {
    /// Create a new flow builder.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            initial_state: None,
            name: "simple_flow".to_string(),
        }
    }

    /// Set the flow name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add a node for a specific state.
    pub fn node(mut self, state: S, node: impl Node<State = S> + 'static) -> Self {
        self.nodes.insert(state, Arc::new(node));
        self
    }

    /// Set the initial state.
    pub fn initial_state(mut self, state: S) -> Self {
        self.initial_state = Some(state);
        self
    }

    /// Build the flow.
    pub fn build(self) -> Result<SimpleFlow<S>> {
        let initial_state = self
            .initial_state
            .ok_or_else(|| FlowError::construction("Initial state not set"))?;

        if self.nodes.is_empty() {
            return Err(FlowError::construction("No nodes added to flow"));
        }

        Ok(SimpleFlow {
            nodes: self.nodes,
            initial_state,
            name: self.name,
        })
    }
}

impl<S: FlowState> Default for SimpleFlowBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
