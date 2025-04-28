use std::sync::Arc;

use dashmap::DashMap;
use eyre::Result;
use futures::future::join_all;

use crate::{
    node::base::AsyncBaseNode,
    types::{ActionKey, DEFAULT_ACTION, Params, SharedState},
};

/// Async flow structure
pub struct AsyncFlow {
    params: Params,
    pub(crate) start_node: Option<Arc<dyn AsyncBaseNode>>,
    node_map: DashMap<ActionKey, Arc<dyn AsyncBaseNode>>,
}

impl Default for AsyncFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncFlow {
    pub fn new() -> Self {
        Self {
            params: Default::default(),
            start_node: None,
            node_map: Default::default(),
        }
    }

    pub fn set_params(&mut self, params: Params) -> &mut Self {
        self.params = params;
        self
    }

    pub fn start(&mut self, node: Arc<dyn AsyncBaseNode>) -> &mut Self {
        self.start_node = Some(node);
        self
    }

    pub async fn run_async(&self, shared: &SharedState) -> Result<String> {
        let start_node = self
            .start_node
            .as_ref()
            .ok_or_else(|| eyre::eyre!("AsyncFlow has no start node"))?;

        let mut current_node = start_node.clone();
        let _params = self.params.clone();

        // Initialize action_result directly when it's first needed
        let mut prep_res = current_node.prep_async(shared).await?;
        let mut exec_res = current_node.exec_async(&prep_res).await?;
        let mut action_result = current_node
            .post_async(shared, &prep_res, &exec_res)
            .await?;

        loop {
            // Get successor from node's successors map
            let next_node_key = if action_result.is_empty() {
                DEFAULT_ACTION.to_string()
            } else {
                action_result.clone()
            };

            // Find the next node
            let next_node_opt = self.node_map.get(&next_node_key).map(|node| node.clone());

            // If there's no next node, break the loop
            if next_node_opt.is_none() {
                break;
            }

            // Update current node and continue execution
            current_node = next_node_opt.expect("Next node not found in flow");
            prep_res = current_node.prep_async(shared).await?;
            exec_res = current_node.exec_async(&prep_res).await?;
            action_result = current_node
                .post_async(shared, &prep_res, &exec_res)
                .await?;
        }

        Ok(action_result)
    }
}

/// Async batch flow structure
pub struct AsyncBatchFlow {
    pub(crate) inner: AsyncFlow,
}

impl Default for AsyncBatchFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncBatchFlow {
    pub fn new() -> Self {
        Self {
            inner: AsyncFlow::new(),
        }
    }

    pub fn set_params(&mut self, params: Params) -> &mut Self {
        self.inner.set_params(params);
        self
    }

    pub fn start(&mut self, node: Arc<dyn AsyncBaseNode>) -> &mut Self {
        self.inner.start(node);
        self
    }

    pub async fn run_async(&self, shared: &SharedState) -> Result<String> {
        let start_node = self
            .inner
            .start_node
            .as_ref()
            .ok_or_else(|| eyre::eyre!("AsyncBatchFlow has no start node"))?;

        let prep_res = start_node.prep_async(shared).await?;
        let batch_items = prep_res
            .as_array()
            .ok_or_else(|| eyre::eyre!("AsyncBatchFlow prep must return an array"))?;

        for item in batch_items {
            let mut params = self.inner.params.clone();
            if let Some(item_obj) = item.as_object() {
                for (k, v) in item_obj {
                    params.insert(k.clone(), v.clone());
                }
            }

            // Execute flow for each batch item
            self.inner.run_async(shared).await?;
        }

        Ok(DEFAULT_ACTION.to_string())
    }
}

/// Async parallel batch flow structure
pub struct AsyncParallelBatchFlow {
    pub(crate) inner: AsyncFlow,
}

impl Default for AsyncParallelBatchFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncParallelBatchFlow {
    pub fn new() -> Self {
        Self {
            inner: AsyncFlow::new(),
        }
    }

    pub fn set_params(&mut self, params: Params) -> &mut Self {
        self.inner.set_params(params);
        self
    }

    pub fn start(&mut self, node: Arc<dyn AsyncBaseNode>) -> &mut Self {
        self.inner.start(node);
        self
    }

    pub async fn run_async(&self, shared: &SharedState) -> Result<String> {
        let start_node = self
            .inner
            .start_node
            .as_ref()
            .ok_or_else(|| eyre::eyre!("AsyncParallelBatchFlow has no start node"))?;

        let prep_res = start_node.prep_async(shared).await?;
        let batch_items = prep_res
            .as_array()
            .ok_or_else(|| eyre::eyre!("AsyncParallelBatchFlow prep must return an array"))?;

        let mut futures = Vec::with_capacity(batch_items.len());

        for item in batch_items {
            let mut params = self.inner.params.clone();
            if let Some(item_obj) = item.as_object() {
                for (k, v) in item_obj {
                    params.insert(k.clone(), v.clone());
                }
            }

            futures.push(self.inner.run_async(shared));
        }

        join_all(futures).await;

        Ok(DEFAULT_ACTION.to_string())
    }
}
