//! # PocketFlow
//!
//! PocketFlow is a lightweight, composable workflow library for Rust that provides
//! both synchronous and asynchronous execution patterns for data processing pipelines.
//!
//! ## Features
//!
//! - **Composable workflows**: Build complex data processing pipelines from simple nodes
//! - **Synchronous and asynchronous support**: Choose the execution model that fits your needs
//! - **Batch processing**: Process multiple items efficiently
//! - **Parallel execution**: Take advantage of multi-core systems
//!
//! ## Examples
//!
//! ### Simple synchronous flow
//!
//! ```rust
//! use pocketflow_rs::{Flow, Node};
//! use serde_json::json;
//!
//! // Create a simple processing node
//! let node = Node::new(|data| {
//!     // Process the data
//!     let result = json!({"processed": data});
//!     Ok(result)
//! });
//!
//! // Create a flow with the node
//! let mut flow = Flow::new();
//! flow.add_node(node);
//!
//! // Run the flow with input data
//! let input = json!({"input": "value"});
//! let result = flow.run(&input).unwrap();
//! ```
//!
//! ### Asynchronous flow
//!
//! ```rust
//! use pocketflow_rs::{AsyncFlow, AsyncNode};
//! use serde_json::json;
//!
//! # async fn run_example() {
//! // Create an async processing node
//! let node = AsyncNode::new(|data| {
//!     Box::pin(async move {
//!         // Asynchronous processing
//!         let result = json!({"processed": data});
//!         Ok(result)
//!     })
//! });
//!
//! // Create an async flow with the node
//! let mut flow = AsyncFlow::new();
//! flow.add_node(node);
//!
//! // Run the flow with input data
//! let input = json!({"input": "value"});
//! let result = flow.run_async(&input).await.unwrap();
//! # }
//! ```

/// Internal module for flow implementations.
mod flow;
/// Internal module for macros.
mod macros;
/// Internal module for node implementations.
mod node;
/// Internal module for type definitions.
mod types;

// Re-export the public API with more specific exports
/// Asynchronous flow implementations for creating complex data processing pipelines.
pub use flow::async_flow::{AsyncBatchFlow, AsyncFlow, AsyncParallelBatchFlow};
// Re-export flow components with more specific names
/// Synchronous flow implementations for creating data processing pipelines.
pub use flow::sync::{BatchFlow, Flow};
// Re-export node components with more specific names
/// Base node traits that define the core behavior of processing nodes.
pub use node::base::{AsyncBaseNode, BaseNode};
/// Node implementations for both synchronous and asynchronous data processing.
pub use node::{
    async_node::{AsyncBatchNode, AsyncNode, AsyncParallelBatchNode},
    sync::{BatchNode, Node},
};
/// Common types and constants used throughout the library.
pub use types::*;

// Tests remain in the main file for simplicity
#[cfg(test)]
mod tests {
    use crate::types::DEFAULT_ACTION;

    #[test]
    fn test_basic_node() {
        let node = crate::node::sync::Node::new(|_| Ok(serde_json::json!("result")));
        let shared = serde_json::json!({});
        let result = node.run(&shared).unwrap();
        assert_eq!(result, DEFAULT_ACTION);
    }

    #[tokio::test]
    async fn test_async_node() {
        let node = crate::node::async_node::AsyncNode::new(|_| {
            Box::pin(async { Ok(serde_json::json!("result")) })
        });
        let shared = serde_json::json!({});
        let result = node.run_async(&shared).await.unwrap();
        assert_eq!(result, DEFAULT_ACTION);
    }

    // Additional tests would go here
}
