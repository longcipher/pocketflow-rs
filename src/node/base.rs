use async_trait::async_trait;
use eyre::Result;

use crate::types::SharedState;

/// The base trait for all nodes in the flow
pub trait BaseNode: Send + Sync + 'static {
    /// Prepare resources for execution
    fn prep(&self, shared: &SharedState) -> Result<serde_json::Value>;
    /// Execute the node's core functionality
    fn exec(&self, prep_res: &serde_json::Value) -> Result<serde_json::Value>;
    /// Post-process after execution
    fn post(
        &self,
        shared: &SharedState,
        prep_res: &serde_json::Value,
        exec_res: &serde_json::Value,
    ) -> Result<String>;
    /// Run the node's full lifecycle
    fn run(&self, shared: &SharedState) -> Result<String> {
        let prep_res = self.prep(shared)?;
        let exec_res = self.exec(&prep_res)?;
        self.post(shared, &prep_res, &exec_res)
    }
}

/// The base trait for all async nodes
#[async_trait]
pub trait AsyncBaseNode: Send + Sync + 'static {
    /// Prepare resources for async execution
    async fn prep_async(&self, shared: &SharedState) -> Result<serde_json::Value>;
    /// Execute the node's core functionality asynchronously
    async fn exec_async(&self, prep_res: &serde_json::Value) -> Result<serde_json::Value>;
    /// Post-process after async execution
    async fn post_async(
        &self,
        shared: &SharedState,
        prep_res: &serde_json::Value,
        exec_res: &serde_json::Value,
    ) -> Result<String>;
    /// Run the node's full async lifecycle
    async fn run_async(&self, shared: &SharedState) -> Result<String> {
        let prep_res = self.prep_async(shared).await?;
        let exec_res = self.exec_async(&prep_res).await?;
        self.post_async(shared, &prep_res, &exec_res).await
    }
}
