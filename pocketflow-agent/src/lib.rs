pub mod agent_node;
pub mod agent_types;
pub mod error;
// pub mod builders;
// pub mod multi_agent;
// pub mod streaming;

// Re-exports for convenience
pub use agent_node::*;
pub use agent_types::*;
// pub use builders::*;
pub use error::*;
// pub use multi_agent::*;
// pub use streaming::*;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        agent_node::{AgentNode, AgentState},
        agent_types::*,
        // builders::*,
        error::{AgentError, Result},
        // multi_agent::{MultiAgentNode, MultiAgentNodeBuilder, CoordinationStrategy},
        // streaming::{StreamingAgentNode, StreamingAgentNodeBuilder, StreamChunk},
    };
}
