use std::sync::Arc;

use dashmap::DashMap;
use eyre::Result;

use crate::{
    node::base::BaseNode,
    types::{ActionKey, DEFAULT_ACTION, Params, SharedState},
};

/// Flow orchestration structure
pub struct Flow {
    params: Params,
    pub(crate) start_node: Option<Arc<dyn BaseNode>>,
    node_map: DashMap<ActionKey, Arc<dyn BaseNode>>,
}

impl Default for Flow {
    fn default() -> Self {
        Self::new()
    }
}

impl Flow {
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

    pub fn start(&mut self, node: Arc<dyn BaseNode>) -> &mut Self {
        self.start_node = Some(node);
        self
    }

    pub fn get_node(&self, action: &str) -> Option<Arc<dyn BaseNode>> {
        self.node_map.get(action).map(|node| node.clone())
    }

    pub fn run(&self, shared: &SharedState) -> Result<String> {
        let start_node = self
            .start_node
            .as_ref()
            .ok_or_else(|| eyre::eyre!("Flow has no start node"))?;

        let mut current_node = start_node.clone();
        let _params = self.params.clone();
        // Initialize action_result directly when it's first needed

        // Start execution
        let mut prep_res = current_node.prep(shared)?;
        let mut exec_res = current_node.exec(&prep_res)?;
        let mut action_result = current_node.post(shared, &prep_res, &exec_res)?;

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
            prep_res = current_node.prep(shared)?;
            exec_res = current_node.exec(&prep_res)?;
            action_result = current_node.post(shared, &prep_res, &exec_res)?;
        }

        Ok(action_result)
    }
}

/// Batch flow structure
pub struct BatchFlow {
    pub(crate) inner: Flow,
}

impl Default for BatchFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchFlow {
    pub fn new() -> Self {
        Self { inner: Flow::new() }
    }

    pub fn set_params(&mut self, params: Params) -> &mut Self {
        self.inner.set_params(params);
        self
    }

    pub fn start(&mut self, node: Arc<dyn BaseNode>) -> &mut Self {
        self.inner.start(node);
        self
    }

    pub fn run(&self, shared: &SharedState) -> Result<String> {
        let prep_res = self
            .inner
            .start_node
            .as_ref()
            .ok_or_else(|| eyre::eyre!("BatchFlow has no start node"))?
            .prep(shared)?;

        let batch_items = prep_res
            .as_array()
            .ok_or_else(|| eyre::eyre!("BatchFlow prep must return an array"))?;

        for item in batch_items {
            let mut params = self.inner.params.clone();
            if let Some(item_obj) = item.as_object() {
                for (k, v) in item_obj {
                    params.insert(k.clone(), v.clone());
                }
            }

            // Execute flow for each batch item
            self.inner.run(shared)?;
        }

        Ok(DEFAULT_ACTION.to_string())
    }
}
