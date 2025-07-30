//! # PocketFlow Agent Framework
//!
//! AI Agent framework with genai integration for building intelligent workflow nodes.
//! This crate extends PocketFlow with AI agent capabilities, allowing workflows to
//! leverage large language models (LLMs) for intelligent decision making and task execution.
//!
//! ## Features
//!
//! - **AgentNode**: LLM-powered workflow nodes with multi-step execution
//! - **Tool Integration**: Seamless integration with `pocketflow-tools` for agent capabilities  
//! - **Execution History**: Complete tracking of agent reasoning and actions
//! - **Multi-Provider Support**: Support for OpenAI and other AI providers via genai
//! - **Streaming Support**: Real-time streaming of agent responses
//! - **Multi-Agent Coordination**: Support for agent-to-agent communication
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//!
//! use pocketflow_agent::prelude::*;
//! use pocketflow_core::prelude::*;
//! use pocketflow_tools::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     // Create agent configuration
//!     let agent_config = AgentConfig {
//!         id: uuid::Uuid::new_v4(),
//!         name: "task_processor".to_string(),
//!         description: "AI task processor".to_string(),
//!         role: AgentRole::Independent,
//!         capabilities: vec![AgentCapability::Basic],
//!         execution_mode: ExecutionMode::Sync,
//!         priority: Priority::Normal,
//!         max_steps: 10,
//!         timeout: None,
//!         model_config: ModelConfig {
//!             provider: ModelProvider::OpenAI,
//!             model_name: "gpt-4o-mini".to_string(),
//!             ..Default::default()
//!         },
//!         system_prompt: "You are a helpful assistant".to_string(),
//!         available_tools: vec![],
//!         metadata: std::collections::HashMap::new(),
//!     };
//!
//!     // Create tool registry with capabilities
//!     let mut tool_registry = ToolRegistry::new();
//!     // Register tools here...
//!
//!     // Create agent node
//!     let agent_node = AgentNode::new(agent_config).with_tools(Arc::new(tool_registry));
//!
//!     // Use in workflow
//!     let mut context = Context::new();
//!     context.set("input", "Process this task with AI")?;
//!
//!     let (result_context, _state) = agent_node.execute(context).await?;
//!     if let Ok(Some(result)) = result_context.get_json::<AgentResult>("agent_result") {
//!         println!("Agent response: {:?}", result.final_answer);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Agent Configuration
//!
//! Agents are configured with:
//! - Model settings (provider, model name, parameters)
//! - System prompts for behavior guidance
//! - Tool registries for available capabilities
//! - Execution limits and timeouts
//!
//! ## Execution Flow
//!
//! 1. Agent receives input through workflow context
//! 2. Agent processes input using configured LLM
//! 3. Agent may call tools based on reasoning
//! 4. Agent returns results through context
//! 5. Execution history is preserved for analysis

pub mod agent_node;
pub mod agent_types;
pub mod builders;
pub mod error;
pub mod multi_agent;
pub mod streaming;

// Re-exports for convenience
pub use agent_node::*;
pub use agent_types::*;
pub use builders::*;
pub use error::*;
pub use multi_agent::*;
pub use streaming::*;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        agent_node::{AgentNode, AgentRegistry, AgentState, ModelAdapter},
        agent_types::*,
        builders::*,
        error::{AgentError, Result},
        multi_agent::{CoordinationStrategy, MultiAgentNode, MultiAgentNodeBuilder},
        streaming::{StreamChunk, StreamingAgentNode, StreamingAgentNodeBuilder},
    };
}
