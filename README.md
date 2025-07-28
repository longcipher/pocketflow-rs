# PocketFlow-RS

A modern workflow framework ecosystem for Rust, providing type-safe, async workflow execution with powerful integrations.

## 📦 Workspace Structure

This is a Cargo workspace containing the following crates:

### [`pocketflow-core`](./pocketflow-core/)

The core workflow framework providing:

- Type-safe state management with compile-time guarantees
- Async/await support built on Tokio
- Flexible context system with typed and JSON storage
- Node-based architecture with dependency injection
- Advanced flows with middleware and analytics

### [`pocketflow-mcp`](./pocketflow-mcp/)

Model Context Protocol (MCP) integration for workflows:

- MCP client integration for calling external tools
- MCP server implementation to expose workflow capabilities
- Seamless context integration between MCP and workflows
- Registry management for multiple connections

## 🚀 Quick Start

### Basic Workflow with Core

```toml
[dependencies]
pocketflow-core = "0.1.0"
```

```rust
use pocketflow_core::prelude::*;
use async_trait::async_trait;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum WorkflowState {
    Start, Processing, Success, Error
}

impl FlowState for WorkflowState {
    fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Success | WorkflowState::Error)
    }
}

struct ProcessingNode;

#[async_trait]
impl Node for ProcessingNode {
    type State = WorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        context.set("result".to_string(), "processed")?;
        Ok((context, WorkflowState::Success))
    }

    fn name(&self) -> String {
        "ProcessingNode".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .add_node(WorkflowState::Start, ProcessingNode)
        .build()?;

    let result = flow.execute(Context::new()).await?;
    println!("Final state: {:?}", result.final_state);
    Ok(())
}
```

### Workflow with MCP Integration

```toml
[dependencies]
pocketflow-core = "0.1.0"
pocketflow-mcp = "0.1.0"
```

```rust
use pocketflow_core::prelude::*;
use pocketflow_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create MCP client
    let client = UltraFastMcpClient::new("http://localhost:8080").await?;
    
    // Build workflow with MCP integration
    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .add_node(WorkflowState::Start, McpClientNode::new(
            "ai_tool_caller".to_string(),
            Arc::new(client),
            "summarize_text".to_string(),
            WorkflowState::Processing,
            WorkflowState::Success,
            WorkflowState::Error,
        ))
        .build()?;

    let mut context = Context::new();
    context.set("tool_args".to_string(), serde_json::json!({
        "text": "Long document to summarize..."
    }))?;

    let result = flow.execute(context).await?;
    println!("Summary result: {:?}", result.context.get_json::<String>("tool_result"));
    Ok(())
}
```

## 🏗️ Architecture

```text
┌─────────────────┐    ┌──────────────────┐
│ pocketflow-core │    │ pocketflow-mcp   │
├─────────────────┤    ├──────────────────┤
│ • Node trait    │    │ • MCP Client     │
│ • Context       │    │ • MCP Server     │
│ • FlowState     │    │ • Registry       │
│ • SimpleFlow    │    │ • Context Ext    │
│ • AdvancedFlow  │    │ • MCP Nodes      │
└─────────────────┘    └──────────────────┘
         │                        │
         └───────┬────────────────┘
                 │
    ┌─────────────────────────┐
    │    Your Application     │
    │                         │
    │ • Custom Nodes          │
    │ • Workflow Logic        │
    │ • MCP Integrations      │
    │ • Business Rules        │
    └─────────────────────────┘
```

## 🔧 Development

The workspace is configured with shared dependencies and development tools:

```bash
# Format all code
just format

# Run all lints
just lint

# Test all crates
just test

# Run examples from specific crates
cargo run --example basic --package pocketflow-core
cargo run --example mcp_demo_simple --package pocketflow-mcp
```

## 📋 Features by Crate

### Core Framework Features

- ✅ Type-safe state machines
- ✅ Async workflow execution  
- ✅ Context management (typed + JSON)
- ✅ Node composition patterns
- ✅ Middleware system
- ✅ Analytics and monitoring
- ✅ Batch processing
- ✅ Error handling with eyre

### MCP Integration Features

- ✅ MCP client for tool calling
- ✅ MCP server implementation
- ✅ Workflow context extensions
- ✅ Registry management
- ✅ HTTP transport
- ⏳ WebSocket transport (planned)
- ⏳ Prompt templates (planned)

## 🎯 Use Cases

### Data Processing Pipelines

Use `pocketflow-core` for structured data transformations with state tracking.

### AI Agent Workflows  

Combine both crates to build AI agents that can call external tools via MCP while maintaining workflow state.

### API Orchestration

Chain multiple service calls with error handling and state management.

### Microservice Communication

Use MCP as a protocol for service-to-service communication within workflows.

## 📚 Documentation

- [Core Framework Documentation](./pocketflow-core/README.md)
- [MCP Integration Documentation](./pocketflow-mcp/README.md)
- [API Documentation](https://docs.rs/pocketflow-core)
- [Examples Directory](./pocketflow-core/examples/)

## 🤝 Contributing

Contributions are welcome! Please:

1. Check existing issues and PRs
2. Follow the coding conventions
3. Add tests for new features
4. Update documentation as needed

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
