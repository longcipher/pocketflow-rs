//! # PocketFlow MCP Integration
//!
//! This crate provides Model Context Protocol (MCP) integration for PocketFlow-RS,
//! enabling workflows to both consume external MCP services and expose workflow
//! functionality as MCP services.
//!
//! This module provides seamless integration between PocketFlow workflows and MCP servers,
//! allowing workflows to:
//! - Call external MCP tools as workflow nodes
//! - Run MCP servers that expose workflow functionality
//! - Pass data between workflow context and MCP tools
//! - Chain multiple MCP operations together
//!
//! ## Features
//!
//! - **McpClientNode**: Call external MCP tools from within workflows
//! - **McpServerNode**: Expose workflow functionality as MCP services
//! - **Context Integration**: Seamless integration with PocketFlow context system
//! - **Registry Management**: Manage multiple MCP clients and servers
//!
//! ## Quick Start
//!
//! ### Using MCP Client in Workflow
//!
//! ```rust,no_run
//! use pocketflow_core::prelude::*;
//! use pocketflow_mcp::prelude::*;
//!
//! #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//! enum MyState {
//!     Start,
//!     McpCall,
//!     Completed,
//! }
//!
//! impl FlowState for MyState {
//!     fn is_terminal(&self) -> bool {
//!         matches!(self, MyState::Completed)
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     // Example MCP workflow setup
//!     let registry = McpRegistry::new();
//!     println!("MCP registry created");
//!     Ok(())
//! }
//! ```
//!
//! ### Exposing Workflow as MCP Server
//!
//! ```rust,no_run
//! use pocketflow_core::prelude::*;
//! use pocketflow_mcp::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     // Example MCP server setup
//!     let registry = McpRegistry::new();
//!     println!("MCP server created");
//!     Ok(())
//! }
//! ```
//! }
//! ```

pub mod client;
pub mod context;
pub mod error;
pub mod registry;
pub mod server;

pub use client::*;
pub use context::*;
pub use error::*;
pub use registry::*;
pub use server::*;
// Re-export common MCP types from ultrafast-mcp
pub use ultrafast_mcp::{
    ClientCapabilities, ClientInfo, ListResourcesRequest, ListResourcesResponse, ListToolsRequest,
    ListToolsResponse, PromptsCapability, ReadResourceRequest, ReadResourceResponse, Resource,
    ResourcesCapability, ServerCapabilities, ServerInfo, Tool, ToolCall, ToolContent, ToolResult,
    ToolsCapability,
};

/// Convenient re-exports for MCP integration.
pub mod prelude {
    // Re-export core types that are commonly used with MCP
    pub use pocketflow_core::prelude::*;
    pub use ultrafast_mcp::{
        ClientInfo, ListResourcesRequest, ListResourcesResponse, ListToolsRequest,
        ListToolsResponse, ReadResourceRequest, ReadResourceResponse, Resource, ServerInfo, Tool,
        ToolCall, ToolContent, ToolResult,
    };

    pub use crate::{
        client::{McpClient, McpClientNode},
        context::{McpContext, McpContextExt},
        error::{McpError, Result},
        registry::McpRegistry,
        server::{
            McpServerConfig, McpToolNode, WorkflowExecutionParams, WorkflowExecutionResult,
            WorkflowMcpHandler, WorkflowStatus,
        },
    };
}
