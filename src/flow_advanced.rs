//! Advanced flow orchestration with enhanced features and middleware support.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

use crate::{
    context::Context,
    error::{FlowError, Result},
    node::Node,
    state::FlowState,
};

/// Advanced flow execution result with enhanced metadata.
#[derive(Debug, Clone)]
pub struct AdvancedFlowResult<S: FlowState> {
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
    /// Execution metadata.
    pub metadata: HashMap<String, String>,
    /// Step-by-step execution trace.
    pub trace: Vec<ExecutionStep<S>>,
}

/// Individual execution step information.
#[derive(Debug, Clone)]
pub struct ExecutionStep<S: FlowState> {
    pub step_number: usize,
    pub from_state: S,
    pub to_state: S,
    pub node_name: String,
    pub duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Middleware function type.
pub type Middleware<S> = Arc<dyn Fn(&Context, &S) -> Result<()> + Send + Sync>;

/// Conditional function type.
pub type Condition<S> = Arc<dyn Fn(&Context, &S) -> bool + Send + Sync>;

/// Advanced workflow execution engine with middleware support.
pub struct AdvancedFlow<S: FlowState> {
    nodes: HashMap<S, Arc<dyn Node<State = S>>>,
    initial_state: S,
    name: String,
    middleware: Vec<Middleware<S>>,
    conditions: HashMap<S, (Condition<S>, S, S)>, // state -> (condition, true_state, false_state)
    max_steps: usize,
}

impl<S: FlowState> AdvancedFlow<S> {
    /// Execute the workflow with middleware support.
    pub async fn execute(&self, mut context: Context) -> Result<AdvancedFlowResult<S>> {
        let start_time = Instant::now();
        let mut current_state = self.initial_state.clone();
        let mut steps = 0;
        let mut trace = Vec::new();
        let mut metadata = HashMap::new();

        metadata.insert("flow_name".to_string(), self.name.clone());
        metadata.insert("started_at".to_string(), chrono::Utc::now().to_rfc3339());

        loop {
            steps += 1;

            // Prevent infinite loops
            if steps > self.max_steps {
                return Ok(AdvancedFlowResult {
                    final_state: current_state,
                    context,
                    duration: start_time.elapsed(),
                    steps,
                    success: false,
                    error: Some(format!("Flow exceeded maximum steps ({})", self.max_steps)),
                    metadata,
                    trace,
                });
            }

            // Check if we've reached a terminal state
            if current_state.is_terminal() {
                metadata.insert("completed_at".to_string(), chrono::Utc::now().to_rfc3339());
                return Ok(AdvancedFlowResult {
                    final_state: current_state,
                    context,
                    duration: start_time.elapsed(),
                    steps,
                    success: true,
                    error: None,
                    metadata,
                    trace,
                });
            }

            // Run pre-execution middleware
            for middleware in &self.middleware {
                if let Err(e) = middleware(&context, &current_state) {
                    return Ok(AdvancedFlowResult {
                        final_state: current_state,
                        context,
                        duration: start_time.elapsed(),
                        steps,
                        success: false,
                        error: Some(format!("Middleware error: {e}")),
                        metadata,
                        trace,
                    });
                }
            }

            let step_start = Instant::now();
            let from_state = current_state.clone();

            // Check for conditional routing
            if let Some((condition, true_state, false_state)) = self.conditions.get(&current_state)
            {
                let next_state = if condition(&context, &current_state) {
                    true_state.clone()
                } else {
                    false_state.clone()
                };

                let step = ExecutionStep {
                    step_number: steps,
                    from_state: from_state.clone(),
                    to_state: next_state.clone(),
                    node_name: "conditional_router".to_string(),
                    duration: step_start.elapsed(),
                    timestamp: chrono::Utc::now(),
                };
                trace.push(step);

                current_state = next_state;
                continue;
            }

            // Find the node for the current state
            let node = self.nodes.get(&current_state).ok_or_else(|| {
                FlowError::execution(format!("No node found for state: {current_state:?}"))
            })?;

            // Execute the node
            match node.execute(context.clone()).await {
                Ok((new_context, new_state)) => {
                    let step = ExecutionStep {
                        step_number: steps,
                        from_state: from_state.clone(),
                        to_state: new_state.clone(),
                        node_name: node.name(),
                        duration: step_start.elapsed(),
                        timestamp: chrono::Utc::now(),
                    };
                    trace.push(step);

                    context = new_context;
                    current_state = new_state;
                }
                Err(error) => {
                    let step = ExecutionStep {
                        step_number: steps,
                        from_state: from_state.clone(),
                        to_state: current_state.clone(),
                        node_name: node.name(),
                        duration: step_start.elapsed(),
                        timestamp: chrono::Utc::now(),
                    };
                    trace.push(step);

                    return Ok(AdvancedFlowResult {
                        final_state: current_state,
                        context,
                        duration: start_time.elapsed(),
                        steps,
                        success: false,
                        error: Some(error.to_string()),
                        metadata,
                        trace,
                    });
                }
            }
        }
    }

    /// Get the flow name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Create a new flow builder.
    pub fn builder() -> AdvancedFlowBuilder<S> {
        AdvancedFlowBuilder::new()
    }
}

/// Builder for constructing advanced flows.
pub struct AdvancedFlowBuilder<S: FlowState> {
    nodes: HashMap<S, Arc<dyn Node<State = S>>>,
    initial_state: Option<S>,
    name: String,
    middleware: Vec<Middleware<S>>,
    conditions: HashMap<S, (Condition<S>, S, S)>,
    max_steps: usize,
}

impl<S: FlowState> AdvancedFlowBuilder<S> {
    /// Create a new flow builder.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            initial_state: None,
            name: "advanced_flow".to_string(),
            middleware: Vec::new(),
            conditions: HashMap::new(),
            max_steps: 1000,
        }
    }

    /// Set the flow name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the initial state.
    pub fn initial_state(mut self, state: S) -> Self {
        self.initial_state = Some(state);
        self
    }

    /// Set maximum number of execution steps.
    pub fn max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Add a node for a specific state.
    pub fn on_state(mut self, state: S, node: impl Node<State = S> + 'static) -> Self {
        self.nodes.insert(state, Arc::new(node));
        self
    }

    /// Add middleware that runs before each node execution.
    pub fn middleware<F>(mut self, middleware: F) -> Self
    where
        F: Fn(&Context, &S) -> Result<()> + Send + Sync + 'static,
    {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Add logging middleware.
    pub fn with_logging(self) -> Self {
        self.middleware(|_context, state| {
            println!("🔄 Executing state: {state:?}");
            Ok(())
        })
    }

    /// Add timing middleware.
    pub fn with_timing(self) -> Self {
        self.middleware(|_context, state| {
            let timestamp = chrono::Utc::now().to_rfc3339();
            // Note: We can't modify context here since it's a reference
            // In a real implementation, you might want to use a different approach
            println!("⏱️  State {state:?} at {timestamp}");
            Ok(())
        })
    }

    /// Add conditional routing for a state.
    pub fn when_state<F>(mut self, state: S, condition: F, true_state: S, false_state: S) -> Self
    where
        F: Fn(&Context, &S) -> bool + Send + Sync + 'static,
    {
        self.conditions
            .insert(state, (Arc::new(condition), true_state, false_state));
        self
    }

    /// Build the flow.
    pub fn build(self) -> Result<AdvancedFlow<S>> {
        let initial_state = self
            .initial_state
            .ok_or_else(|| FlowError::construction("Initial state not set"))?;

        if self.nodes.is_empty() && self.conditions.is_empty() {
            return Err(FlowError::construction(
                "No nodes or conditions added to flow",
            ));
        }

        Ok(AdvancedFlow {
            nodes: self.nodes,
            initial_state,
            name: self.name,
            middleware: self.middleware,
            conditions: self.conditions,
            max_steps: self.max_steps,
        })
    }
}

impl<S: FlowState> Default for AdvancedFlowBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared flow state for concurrent access.
pub struct SharedFlowState<S: FlowState> {
    state: Arc<RwLock<S>>,
    context: Arc<RwLock<Context>>,
    metadata: Arc<RwLock<HashMap<String, String>>>,
}

impl<S: FlowState> SharedFlowState<S> {
    pub fn new(initial_state: S, context: Context) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial_state)),
            context: Arc::new(RwLock::new(context)),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_state(&self) -> S {
        self.state.read().await.clone()
    }

    pub async fn set_state(&self, new_state: S) {
        *self.state.write().await = new_state;
    }

    pub async fn get_context(&self) -> Context {
        self.context.read().await.clone()
    }

    pub async fn update_context<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Context) -> Result<()>,
    {
        let mut context = self.context.write().await;
        f(&mut context)
    }

    pub async fn set_metadata(&self, key: String, value: String) {
        self.metadata.write().await.insert(key, value);
    }

    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata.read().await.get(key).cloned()
    }
}

/// Flow registry for managing multiple flows.
pub struct FlowRegistry<S: FlowState> {
    flows: HashMap<String, AdvancedFlow<S>>,
}

impl<S: FlowState> FlowRegistry<S> {
    pub fn new() -> Self {
        Self {
            flows: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, flow: AdvancedFlow<S>) {
        self.flows.insert(name, flow);
    }

    pub fn get(&self, name: &str) -> Option<&AdvancedFlow<S>> {
        self.flows.get(name)
    }

    pub async fn execute(&self, name: &str, context: Context) -> Result<AdvancedFlowResult<S>> {
        let flow = self
            .get(name)
            .ok_or_else(|| FlowError::construction(format!("Flow '{name}' not found")))?;

        flow.execute(context).await
    }

    pub fn list_flows(&self) -> Vec<&str> {
        self.flows.keys().map(|s| s.as_str()).collect()
    }
}

impl<S: FlowState> Default for FlowRegistry<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Flow analytics and metrics.
pub struct FlowAnalytics<S: FlowState> {
    execution_history: Vec<AdvancedFlowResult<S>>,
}

impl<S: FlowState> FlowAnalytics<S> {
    pub fn new() -> Self {
        Self {
            execution_history: Vec::new(),
        }
    }

    pub fn record_execution(&mut self, result: AdvancedFlowResult<S>) {
        self.execution_history.push(result);
    }

    pub fn success_rate(&self) -> f64 {
        if self.execution_history.is_empty() {
            return 0.0;
        }

        let successful = self.execution_history.iter().filter(|r| r.success).count();

        successful as f64 / self.execution_history.len() as f64
    }

    pub fn average_execution_time(&self) -> Duration {
        if self.execution_history.is_empty() {
            return Duration::from_secs(0);
        }

        let total: Duration = self.execution_history.iter().map(|r| r.duration).sum();

        total / self.execution_history.len() as u32
    }

    pub fn average_steps(&self) -> f64 {
        if self.execution_history.is_empty() {
            return 0.0;
        }

        let total_steps: usize = self.execution_history.iter().map(|r| r.steps).sum();

        total_steps as f64 / self.execution_history.len() as f64
    }

    pub fn most_common_final_state(&self) -> Option<&S> {
        let mut state_counts = HashMap::new();

        for result in &self.execution_history {
            *state_counts.entry(&result.final_state).or_insert(0) += 1;
        }

        state_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(state, _)| state)
    }
}

impl<S: FlowState> Default for FlowAnalytics<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum TestState {
        Start,
        Middle,
        End,
    }

    impl FlowState for TestState {
        fn is_terminal(&self) -> bool {
            matches!(self, TestState::End)
        }
    }

    #[derive(Debug)]
    struct TestNode(TestState);

    #[async_trait]
    impl Node for TestNode {
        type State = TestState;

        async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
            Ok((context, self.0.clone()))
        }

        fn name(&self) -> String {
            format!("test_node_{:?}", self.0)
        }
    }

    #[tokio::test]
    async fn test_advanced_flow_builder() {
        let flow = AdvancedFlow::builder()
            .name("test_flow")
            .initial_state(TestState::Start)
            .on_state(TestState::Start, TestNode(TestState::Middle))
            .on_state(TestState::Middle, TestNode(TestState::End))
            .with_logging()
            .build()
            .unwrap();

        let context = Context::new();
        let result = flow.execute(context).await.unwrap();

        assert_eq!(result.final_state, TestState::End);
        assert!(result.success);
        assert_eq!(result.steps, 2);
        assert_eq!(result.trace.len(), 2);
    }

    #[tokio::test]
    async fn test_conditional_routing() {
        let flow = AdvancedFlow::builder()
            .name("conditional_test")
            .initial_state(TestState::Start)
            .when_state(
                TestState::Start,
                |context, _state| {
                    context
                        .get_json::<bool>("should_skip")
                        .unwrap_or(Some(false))
                        .unwrap_or(false)
                },
                TestState::End,    // true: skip to end
                TestState::Middle, // false: go to middle
            )
            .on_state(TestState::Middle, TestNode(TestState::End))
            .build()
            .unwrap();

        // Test with skip condition false
        let mut context = Context::new();
        context.set("should_skip", false).unwrap();
        let result = flow.execute(context).await.unwrap();
        assert_eq!(result.steps, 2); // Start -> Middle -> End

        // Test with skip condition true
        let mut context = Context::new();
        context.set("should_skip", true).unwrap();
        let result = flow.execute(context).await.unwrap();
        assert_eq!(result.steps, 1); // Start -> End (skipped Middle)
    }

    #[tokio::test]
    async fn test_flow_registry() {
        let mut registry = FlowRegistry::new();

        let flow = AdvancedFlow::builder()
            .name("registry_test")
            .initial_state(TestState::Start)
            .on_state(TestState::Start, TestNode(TestState::End))
            .build()
            .unwrap();

        registry.register("test_flow".to_string(), flow);

        let context = Context::new();
        let result = registry.execute("test_flow", context).await.unwrap();

        assert_eq!(result.final_state, TestState::End);
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_flow_analytics() {
        let mut analytics = FlowAnalytics::new();

        // Record some successful executions
        for _ in 0..8 {
            let result = AdvancedFlowResult {
                final_state: TestState::End,
                context: Context::new(),
                duration: Duration::from_millis(100),
                steps: 2,
                success: true,
                error: None,
                metadata: HashMap::new(),
                trace: Vec::new(),
            };
            analytics.record_execution(result);
        }

        // Record some failed executions
        for _ in 0..2 {
            let result = AdvancedFlowResult {
                final_state: TestState::Middle,
                context: Context::new(),
                duration: Duration::from_millis(50),
                steps: 1,
                success: false,
                error: Some("Test error".to_string()),
                metadata: HashMap::new(),
                trace: Vec::new(),
            };
            analytics.record_execution(result);
        }

        assert_eq!(analytics.success_rate(), 0.8);
        assert_eq!(analytics.average_steps(), 1.8);
        assert_eq!(analytics.most_common_final_state(), Some(&TestState::End));
    }
}
