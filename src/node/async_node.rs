use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use dashmap::DashMap;
use eyre::Result;
use futures::future::join_all;
use tracing::warn;

use crate::{
    node::base::AsyncBaseNode,
    types::{ActionKey, DEFAULT_ACTION, Params, SharedState},
};

/// Async node implementation
pub struct AsyncNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    params: Params,
    successors: DashMap<ActionKey, Arc<dyn AsyncBaseNode>>,
    exec_fn: F,
    max_retries: usize,
    wait_ms: u64,
}

impl<F> AsyncNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    pub fn new(exec_fn: F) -> Self {
        Self {
            params: Default::default(),
            successors: DashMap::new(),
            exec_fn,
            max_retries: 1,
            wait_ms: 0,
        }
    }

    pub fn set_params(&mut self, params: Params) -> &mut Self {
        self.params = params;
        self
    }

    pub fn max_retries(&mut self, retries: usize) -> &mut Self {
        self.max_retries = retries;
        self
    }

    pub fn wait_ms(&mut self, wait: u64) -> &mut Self {
        self.wait_ms = wait;
        self
    }

    pub fn next(&mut self, node: Arc<dyn AsyncBaseNode>, action: &str) -> &mut Self {
        let action_key = action.to_string();
        if self.successors.contains_key(&action_key) {
            warn!("Overwriting successor for action '{}'", action);
        }
        self.successors.insert(action_key, node);
        self
    }

    pub fn get_param<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.params
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

#[async_trait]
impl<F> AsyncBaseNode for AsyncNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    async fn prep_async(&self, _shared: &SharedState) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(&self.params)?)
    }

    async fn exec_async(&self, prep_res: &serde_json::Value) -> Result<serde_json::Value> {
        let mut last_error = None;

        for retry in 0..self.max_retries {
            match (self.exec_fn)(prep_res).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if retry < self.max_retries - 1 && self.wait_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(self.wait_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| eyre::eyre!("Execution failed with unknown error")))
    }

    async fn post_async(
        &self,
        _shared: &SharedState,
        _prep_res: &serde_json::Value,
        _exec_res: &serde_json::Value,
    ) -> Result<String> {
        Ok(DEFAULT_ACTION.to_string())
    }
}

/// Async batch node implementation
pub struct AsyncBatchNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    inner: AsyncNode<F>,
}

impl<F> AsyncBatchNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    pub fn new(exec_fn: F) -> Self {
        Self {
            inner: AsyncNode::new(exec_fn),
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

    pub fn next(&mut self, node: Arc<dyn AsyncBaseNode>, action: &str) -> &mut Self {
        self.inner.next(node, action);
        self
    }
}

#[async_trait]
impl<F> AsyncBaseNode for AsyncBatchNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    async fn prep_async(&self, shared: &SharedState) -> Result<serde_json::Value> {
        self.inner.prep_async(shared).await
    }

    async fn exec_async(&self, prep_res: &serde_json::Value) -> Result<serde_json::Value> {
        let items = prep_res
            .as_array()
            .ok_or_else(|| eyre::eyre!("Batch input must be an array"))?;

        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(self.inner.exec_async(item).await?);
        }

        Ok(serde_json::to_value(results)?)
    }

    async fn post_async(
        &self,
        shared: &SharedState,
        prep_res: &serde_json::Value,
        exec_res: &serde_json::Value,
    ) -> Result<String> {
        self.inner.post_async(shared, prep_res, exec_res).await
    }
}

/// Async parallel batch node implementation
pub struct AsyncParallelBatchNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    inner: AsyncNode<F>,
}

impl<F> AsyncParallelBatchNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + 'static,
{
    pub fn new(exec_fn: F) -> Self {
        Self {
            inner: AsyncNode::new(exec_fn),
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

    pub fn next(&mut self, node: Arc<dyn AsyncBaseNode>, action: &str) -> &mut Self {
        self.inner.next(node, action);
        self
    }
}

#[async_trait]
impl<F> AsyncBaseNode for AsyncParallelBatchNode<F>
where
    F: for<'a> Fn(
            &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>,
        > + Send
        + Sync
        + Clone
        + 'static,
{
    async fn prep_async(&self, shared: &SharedState) -> Result<serde_json::Value> {
        self.inner.prep_async(shared).await
    }

    async fn exec_async(&self, prep_res: &serde_json::Value) -> Result<serde_json::Value> {
        let items = prep_res
            .as_array()
            .ok_or_else(|| eyre::eyre!("Batch input must be an array"))?;

        let futures = items.iter().map(|item| self.inner.exec_async(item));

        let results = join_all(futures).await;
        let mut processed_results = Vec::with_capacity(results.len());

        for result in results {
            processed_results.push(result?);
        }

        Ok(serde_json::to_value(processed_results)?)
    }

    async fn post_async(
        &self,
        shared: &SharedState,
        prep_res: &serde_json::Value,
        exec_res: &serde_json::Value,
    ) -> Result<String> {
        self.inner.post_async(shared, prep_res, exec_res).await
    }
}
