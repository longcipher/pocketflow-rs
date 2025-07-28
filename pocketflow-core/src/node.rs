//! Node abstraction for PocketFlow workflows.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::{context::Context, error::Result, state::FlowState};

/// Trait for workflow nodes.
///
/// A node represents a unit of work in a workflow. It takes a context,
/// performs some operation, and returns an updated context along with
/// the next state.
#[async_trait]
pub trait Node: Send + Sync + Debug {
    /// The state type this node works with.
    type State: FlowState;

    /// Execute the node's logic.
    ///
    /// This method receives the current context and should return:
    /// - The updated context
    /// - The next state to transition to
    async fn execute(&self, context: Context) -> Result<(Context, Self::State)>;

    /// Optional preparation step before execution.
    ///
    /// This can be used for validation, setup, or other preprocessing.
    async fn prepare(&self, context: &Context) -> Result<()> {
        let _ = context;
        Ok(())
    }

    /// Optional cleanup step after execution.
    ///
    /// This can be used for cleanup, logging, or other postprocessing.
    async fn cleanup(&self, context: &Context, state: &Self::State) -> Result<()> {
        let _ = (context, state);
        Ok(())
    }

    /// Get the name of this node for debugging/logging.
    fn name(&self) -> String {
        format!("{self:?}")
    }
}

/// A simple functional node that wraps a closure.
pub struct FnNode<F, S>
where
    F: Fn(
            Context,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    S: FlowState,
{
    func: F,
    name: String,
    _phantom: std::marker::PhantomData<S>,
}

impl<F, S> std::fmt::Debug for FnNode<F, S>
where
    F: Fn(
            Context,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    S: FlowState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnNode").field("name", &self.name).finish()
    }
}

impl<F, S> FnNode<F, S>
where
    F: Fn(
            Context,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    S: FlowState,
{
    /// Create a new functional node.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            func,
            name: name.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<F, S> Node for FnNode<F, S>
where
    F: Fn(
            Context,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    S: FlowState,
{
    type State = S;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        (self.func)(context).await
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// A no-op node that simply transitions to a specified state.
#[derive(Debug, Clone)]
pub struct PassthroughNode<S: FlowState> {
    target_state: S,
    name: String,
}

impl<S: FlowState> PassthroughNode<S> {
    /// Create a new passthrough node.
    pub fn new(name: impl Into<String>, target_state: S) -> Self {
        Self {
            target_state,
            name: name.into(),
        }
    }
}

#[async_trait]
impl<S: FlowState> Node for PassthroughNode<S> {
    type State = S;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        Ok((context, self.target_state.clone()))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// A conditional node that chooses between states based on a predicate.
pub struct ConditionalNode<F, S>
where
    F: Fn(&Context) -> bool + Send + Sync,
    S: FlowState,
{
    condition: F,
    true_state: S,
    false_state: S,
    name: String,
    _phantom: std::marker::PhantomData<S>,
}

impl<F, S> std::fmt::Debug for ConditionalNode<F, S>
where
    F: Fn(&Context) -> bool + Send + Sync,
    S: FlowState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionalNode")
            .field("name", &self.name)
            .field("true_state", &self.true_state)
            .field("false_state", &self.false_state)
            .finish()
    }
}

impl<F, S> ConditionalNode<F, S>
where
    F: Fn(&Context) -> bool + Send + Sync,
    S: FlowState,
{
    /// Create a new conditional node.
    pub fn new(name: impl Into<String>, condition: F, true_state: S, false_state: S) -> Self {
        Self {
            condition,
            true_state,
            false_state,
            name: name.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<F, S> Node for ConditionalNode<F, S>
where
    F: Fn(&Context) -> bool + Send + Sync,
    S: FlowState,
{
    type State = S;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        let state = if (self.condition)(&context) {
            self.true_state.clone()
        } else {
            self.false_state.clone()
        };
        Ok((context, state))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// A batch processing node that applies a function to a collection of items.
pub struct BatchNode<F, T, S>
where
    F: Fn(
            Context,
            Vec<T>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    T: Send + Sync + 'static,
    S: FlowState,
{
    func: F,
    name: String,
    items_key: String,
    _phantom: std::marker::PhantomData<(T, S)>,
}

impl<F, T, S> std::fmt::Debug for BatchNode<F, T, S>
where
    F: Fn(
            Context,
            Vec<T>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    T: Send + Sync + 'static,
    S: FlowState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchNode")
            .field("name", &self.name)
            .finish()
    }
}

impl<F, T, S> BatchNode<F, T, S>
where
    F: Fn(
            Context,
            Vec<T>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    T: Send + Sync + 'static,
    S: FlowState,
{
    /// Create a new batch processing node.
    pub fn new(name: impl Into<String>, items_key: impl Into<String>, func: F) -> Self {
        Self {
            func,
            items_key: items_key.into(),
            name: name.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<F, T, S> Node for BatchNode<F, T, S>
where
    F: Fn(
            Context,
            Vec<T>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
    T: Send + Sync + serde::de::DeserializeOwned + 'static,
    S: FlowState,
{
    type State = S;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        let items: Vec<T> = context.get_json(&self.items_key)?.unwrap_or_default();

        (self.func)(context, items).await
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Helper functions for creating common node types.
pub mod helpers {
    use super::*;
    use crate::state::FlowState;

    /// Create a functional node from an async closure.
    #[allow(clippy::type_complexity)]
    pub fn fn_node<F, Fut, S>(
        name: impl Into<String>,
        f: F,
    ) -> FnNode<
        impl Fn(
            Context,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Context, S)>> + Send>>
        + Send
        + Sync,
        S,
    >
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(Context, S)>> + Send + 'static,
        S: FlowState,
    {
        FnNode::new(name, move |ctx| Box::pin(f(ctx)))
    }

    /// Create a passthrough node that transitions to a specific state.
    pub fn passthrough<S: FlowState>(name: impl Into<String>, state: S) -> PassthroughNode<S> {
        PassthroughNode::new(name, state)
    }

    /// Create a conditional node.
    pub fn conditional<F, S>(
        name: impl Into<String>,
        condition: F,
        true_state: S,
        false_state: S,
    ) -> ConditionalNode<F, S>
    where
        F: Fn(&Context) -> bool + Send + Sync,
        S: FlowState,
    {
        ConditionalNode::new(name, condition, true_state, false_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SimpleState;

    #[derive(Debug)]
    struct TestNode {
        name: String,
        target_state: SimpleState,
    }

    impl TestNode {
        fn new(name: impl Into<String>, target_state: SimpleState) -> Self {
            Self {
                name: name.into(),
                target_state,
            }
        }
    }

    #[async_trait]
    impl Node for TestNode {
        type State = SimpleState;

        async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
            context.set("executed", true)?;
            context.set("node_name", &self.name)?;
            Ok((context, self.target_state.clone()))
        }

        fn name(&self) -> String {
            self.name.clone()
        }
    }

    #[tokio::test]
    async fn test_basic_node() {
        let node = TestNode::new("test", SimpleState::Success);
        let context = Context::new();

        let (result_context, state) = node.execute(context).await.unwrap();

        assert_eq!(state, SimpleState::Success);
        assert_eq!(
            result_context.get_json::<bool>("executed").unwrap(),
            Some(true)
        );
        assert_eq!(
            result_context.get_json::<String>("node_name").unwrap(),
            Some("test".to_string())
        );
    }

    #[tokio::test]
    async fn test_passthrough_node() {
        let node = helpers::passthrough("passthrough", SimpleState::Processing);
        let context = Context::new();

        let (result_context, state) = node.execute(context).await.unwrap();

        assert_eq!(state, SimpleState::Processing);
        // Context should be unchanged
        assert!(!result_context.contains_json("executed"));
    }

    #[tokio::test]
    async fn test_conditional_node() {
        let node = helpers::conditional(
            "conditional",
            |ctx: &Context| {
                ctx.get_json::<bool>("condition")
                    .unwrap_or(Some(false))
                    .unwrap_or(false)
            },
            SimpleState::Success,
            SimpleState::Error,
        );

        // Test true condition
        let mut context = Context::new();
        context.set("condition", true).unwrap();
        let (_, state) = node.execute(context).await.unwrap();
        assert_eq!(state, SimpleState::Success);

        // Test false condition
        let mut context = Context::new();
        context.set("condition", false).unwrap();
        let (_, state) = node.execute(context).await.unwrap();
        assert_eq!(state, SimpleState::Error);
    }

    #[tokio::test]
    async fn test_fn_node() {
        let node = helpers::fn_node("fn_node", |mut ctx: Context| async move {
            ctx.set("processed", true)?;
            Ok((ctx, SimpleState::Success))
        });

        let context = Context::new();
        let (result_context, state) = node.execute(context).await.unwrap();

        assert_eq!(state, SimpleState::Success);
        assert_eq!(
            result_context.get_json::<bool>("processed").unwrap(),
            Some(true)
        );
    }
}
