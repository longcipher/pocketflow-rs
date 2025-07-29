//! # PocketFlow Cognitive Extensions
//!
//! This crate extends PocketFlow with cognitive capabilities including thinking,
//! planning, reasoning, and memory management without modifying the core framework.
//!
//! ## Core Design Principles
//!
//! - **Non-intrusive Extension**: Extends existing traits without modification
//! - **Composition Over Inheritance**: Wraps and composes existing Node types
//! - **MCP Integration**: Leverages Model Context Protocol for AI services
//! - **Memory Management**: Multi-layered memory systems for context retention
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//!
//! use async_trait::async_trait;
//! use pocketflow_cognitive::prelude::*;
//! use pocketflow_core::{flow::SimpleFlow, prelude::*};
//! use serde_json::Value;
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! enum MyState {
//!     Start,
//!     Planning,
//!     Complete,
//! }
//!
//! impl FlowState for MyState {
//!     fn is_terminal(&self) -> bool {
//!         matches!(self, MyState::Complete)
//!     }
//! }
//!
//! // Mock MCP client for testing
//! struct MockClient;
//!
//! #[async_trait]
//! impl pocketflow_mcp::client::McpClient for MockClient {
//!     async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> {
//!         Ok(vec![])
//!     }
//!
//!     async fn call_tool(&self, _name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
//!         Ok(Value::String("Mock response".to_string()))
//!     }
//!
//!     async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> {
//!         Ok(vec![])
//!     }
//!
//!     async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> {
//!         Ok(Value::String("Mock resource".to_string()))
//!     }
//!
//!     async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> {
//!         Ok(pocketflow_mcp::ServerInfo::new(
//!             "mock".to_string(),
//!             "1.0.0".to_string(),
//!         ))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> pocketflow_core::error::Result<()> {
//!     let mock_client = Arc::new(MockClient);
//!
//!     let planner = GoalOrientedPlanningNode::builder()
//!         .name("my_planner")
//!         .with_mcp_client(mock_client)
//!         .on_success(MyState::Complete)
//!         .on_error(MyState::Complete)
//!         .build()?;
//!
//!     let flow = SimpleFlow::builder()
//!         .initial_state(MyState::Start)
//!         .node(MyState::Start, planner)
//!         .build()?;
//!
//!     let result = flow.execute(Context::new()).await?;
//!     println!("Flow completed with state: {:?}", result.final_state);
//!     Ok(())
//! }
//! ```

pub mod context;
pub mod error;
pub mod memory;
pub mod planning;
pub mod thinking;
pub mod traits;
pub mod utils;

/// Re-export commonly used types and traits
pub mod prelude {
    // Re-export core types for convenience
    pub use pocketflow_core::prelude::*;
    pub use pocketflow_mcp::prelude::*;

    pub use crate::{
        context::CognitiveContextExt,
        error::{CognitiveError, Result},
        memory::{CognitiveMemory, EpisodicMemory, SemanticMemory, WorkingMemory},
        planning::{
            AdaptivePlanningNode, GoalOrientedPlanningNode, HierarchicalPlanningNode,
            PlanningConfig, PlanningStrategy,
        },
        thinking::{ChainOfThoughtNode, ReasoningStrategy, ThinkingConfig},
        traits::{CognitiveNode, ExecutionPlan, Goal, PlanningNode, ThinkingNode},
    };
}

use pocketflow_core::error::Result as CoreResult;

/// Cognitive framework result type
pub type Result<T> = CoreResult<T>;
