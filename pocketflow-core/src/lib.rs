//! # PocketFlow-RS
//!
//! A lightweight, type-safe workflow framework inspired by PocketFlow Python,
//! leveraging Rust's type system for compile-time correctness.
//!
//! ## Core Concepts
//!
//! - **Node**: A unit of work that processes context and produces a state transition
//! - **Context**: Type-safe shared state between nodes  
//! - **State**: Enumeration representing workflow states
//! - **Flow**: Orchestrates nodes and manages state transitions
//!
//! ## Quick Start
//!
//! ```rust
//! use pocketflow_rs::prelude::*;
//!
//! // Define your workflow state
//! #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//! enum WorkflowState {
//!     Start,
//!     Processing,
//!     Success,
//!     Error,
//! }
//!
//! impl FlowState for WorkflowState {
//!     fn is_terminal(&self) -> bool {
//!         matches!(self, WorkflowState::Success | WorkflowState::Error)
//!     }
//! }
//! ```

pub mod context;
pub mod error;
pub mod flow;
pub mod flow_advanced;
pub mod flow_simple;
pub mod node;
pub mod state;

/// Convenient re-exports for common use.
pub mod prelude {
    pub use async_trait::async_trait;
    pub use dptree;
    pub use eyre;
    pub use serde::{Deserialize, Serialize};
    pub use tokio;

    pub use crate::{
        context::{Context, ContextBuilder},
        error::{FlowError, Result},
        flow::{FlowResult, SimpleFlow, SimpleFlowBuilder},
        flow_advanced::{
            AdvancedFlow, AdvancedFlowBuilder, AdvancedFlowResult, FlowRegistry, SharedFlowState,
        },
        node::{BatchNode, ConditionalNode, FnNode, Node, PassthroughNode},
        state::{FlowState, SimpleState},
    };
}
