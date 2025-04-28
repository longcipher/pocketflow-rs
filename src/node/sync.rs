use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use eyre::Result;
use tracing::warn;

use crate::{
    node::base::BaseNode,
    types::{ActionKey, DEFAULT_ACTION, Params, SharedState},
};

/// A node with retry capabilities
pub struct Node<F>
where
    F: Fn(&serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
{
    params: Params,
    successors: DashMap<ActionKey, Arc<dyn BaseNode>>,
    exec_fn: F,
    max_retries: usize,
    wait_ms: u64,
}

impl<F> Node<F>
where
    F: Fn(&serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
{
    /// Create a new node with retry capabilities
    pub fn new(exec_fn: F) -> Self {
        Self {
            params: Default::default(),
            successors: DashMap::new(),
            exec_fn,
            max_retries: 1,
            wait_ms: 0,
        }
    }

    /// Set parameters for this node
    pub fn set_params(&mut self, params: Params) -> &mut Self {
        self.params = params;
        self
    }

    /// Set the max number of retry attempts
    pub fn max_retries(&mut self, retries: usize) -> &mut Self {
        self.max_retries = retries;
        self
    }

    /// Set the wait time between retries in milliseconds
    pub fn wait_ms(&mut self, wait: u64) -> &mut Self {
        self.wait_ms = wait;
        self
    }

    /// Add a successor node for a given action
    pub fn next(&mut self, node: Arc<dyn BaseNode>, action: &str) -> &mut Self {
        let action_key = action.to_string();
        if self.successors.contains_key(&action_key) {
            warn!("Overwriting successor for action '{}'", action);
        }
        self.successors.insert(action_key, node);
        self
    }

    /// Get a parameter value
    pub fn get_param<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.params
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

impl<F> BaseNode for Node<F>
where
    F: Fn(&serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
{
    fn prep(&self, _shared: &SharedState) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(&self.params)?)
    }

    fn exec(&self, prep_res: &serde_json::Value) -> Result<serde_json::Value> {
        let mut last_error = None;

        for retry in 0..self.max_retries {
            match (self.exec_fn)(prep_res) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if retry < self.max_retries - 1 && self.wait_ms > 0 {
                        std::thread::sleep(Duration::from_millis(self.wait_ms));
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| eyre::eyre!("Execution failed with unknown error")))
    }

    fn post(
        &self,
        _shared: &SharedState,
        _prep_res: &serde_json::Value,
        _exec_res: &serde_json::Value,
    ) -> Result<String> {
        // By default, we return the "default" action
        Ok(DEFAULT_ACTION.to_string())
    }
}

/// A node that processes a batch of items
pub struct BatchNode<F>
where
    F: Fn(&serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
{
    inner: Node<F>,
}

impl<F> BatchNode<F>
where
    F: Fn(&serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
{
    pub fn new(exec_fn: F) -> Self {
        Self {
            inner: Node::new(exec_fn),
        }
    }

    pub fn set_params(&mut self, params: Params) -> &mut Self {
        self.inner.set_params(params);
        self
    }

    pub fn max_retries(&mut self, retries: usize) -> &mut Self {
        self.inner.max_retries(retries);
        self
    }

    pub fn wait_ms(&mut self, wait: u64) -> &mut Self {
        self.inner.wait_ms(wait);
        self
    }

    pub fn next(&mut self, node: Arc<dyn BaseNode>, action: &str) -> &mut Self {
        self.inner.next(node, action);
        self
    }
}

impl<F> BaseNode for BatchNode<F>
where
    F: Fn(&serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
{
    fn prep(&self, shared: &SharedState) -> Result<serde_json::Value> {
        self.inner.prep(shared)
    }

    fn exec(&self, prep_res: &serde_json::Value) -> Result<serde_json::Value> {
        let items = prep_res
            .as_array()
            .ok_or_else(|| eyre::eyre!("Batch input must be an array"))?;

        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(self.inner.exec(item)?);
        }

        Ok(serde_json::to_value(results)?)
    }

    fn post(
        &self,
        shared: &SharedState,
        prep_res: &serde_json::Value,
        exec_res: &serde_json::Value,
    ) -> Result<String> {
        self.inner.post(shared, prep_res, exec_res)
    }
}
