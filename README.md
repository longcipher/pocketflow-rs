# PocketFlow-RS

A modern workflow framework ecosystem for Rust, providing type-safe, async workflow execution with powerful integrations including AI agents, cognitive capabilities, and tool automation.

## 📦 Workspace Structure

This is a Cargo workspace containing five specialized crates:

### [`pocketflow-core`](./pocketflow-core/)

The foundation workflow framework providing:

- Type-safe state management with compile-time guarantees
- Async/await support built on Tokio and dptree
- Flexible context system with typed and JSON storage
- Node-based architecture with dependency injection
- Advanced flows with middleware and analytics
- Batch processing and error handling with eyre

### [`pocketflow-mcp`](./pocketflow-mcp/)

Model Context Protocol (MCP) integration for workflows:

- MCP client integration for calling external tools
- MCP server implementation to expose workflow capabilities
- Seamless context integration between MCP and workflows
- Registry management for multiple connections
- HTTP transport with authentication support

### [`pocketflow-cognitive`](./pocketflow-cognitive/)

Cognitive extensions adding AI reasoning capabilities:

- Chain-of-thought and reflection nodes
- Goal-oriented and hierarchical planning
- Multi-layered memory systems (working, episodic, semantic)
- Non-intrusive extension of existing Node types
- MCP integration for AI service calls

### [`pocketflow-agent`](./pocketflow-agent/)

AI Agent framework with genai integration:

- AgentNode implementation for LLM-powered workflows
- Multi-step agent execution with history tracking
- Tool registry integration for agent capabilities
- Streaming and multi-agent coordination support
- Support for multiple AI providers (OpenAI, etc.)

### [`pocketflow-tools`](./pocketflow-tools/)

Comprehensive tool system for workflow automation:

- Tool abstraction with JSON schema validation
- Tool registry for discovery and execution
- Built-in utilities for common operations
- Parameter validation and retry mechanisms
- Integration across the entire ecosystem

## 🚀 Quick Start

### Basic Workflow with Core

```toml
[dependencies]
pocketflow-core = "0.2.0"
```

```rust
use pocketflow_core::prelude::*;

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
        context.set("result", "processed")?;
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
        .node(WorkflowState::Start, ProcessingNode)
        .build()?;

    let result = flow.execute(Context::new()).await?;
    println!("Final state: {:?}", result.final_state);
    Ok(())
}
```

### AI Agent Workflow

```toml
[dependencies]
pocketflow-core = "0.2.0"
pocketflow-agent = "0.2.0"
pocketflow-tools = "0.2.0"
```

```rust
use pocketflow_agent::prelude::*;
use pocketflow_core::prelude::*;
use pocketflow_tools::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create agent configuration
    let agent_config = AgentConfig {
        name: "task_processor".to_string(),
        model_config: ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "gpt-4o-mini".to_string(),
            ..Default::default()
        },
        system_prompt: "You are a helpful task processing agent".to_string(),
        ..Default::default()
    };

    // Create tool registry
    let mut tool_registry = ToolRegistry::new();
    let text_tool = pocketflow_tools::custom::helpers::uppercase_tool();
    tool_registry.register_tool(Box::new(text_tool)).await?;

    // Create agent node with tools
    let agent_node = AgentNode::new(agent_config)
        .with_tools(Arc::new(tool_registry));

    // Use in workflow
    let mut context = Context::new();
    context.set("input", "Process this text with AI")?;
    
    let (result_context, _state) = agent_node.execute(context).await?;
    if let Ok(Some(result)) = result_context.get_json::<AgentResult>("agent_result") {
        println!("Agent response: {:?}", result.final_answer);
    }
    
    Ok(())
}
```

### Cognitive Workflow with Planning

```toml
[dependencies]
pocketflow-core = "0.2.0"
pocketflow-cognitive = "0.2.0"
pocketflow-mcp = "0.2.0"
```

```rust
use pocketflow_cognitive::prelude::*;
use pocketflow_core::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Mock MCP client for cognitive services
    let mcp_client = Arc::new(MockMcpClient::new());
    
    // Create planning node
    let planner = GoalOrientedPlanningNode::builder()
        .name("task_planner")
        .with_mcp_client(mcp_client)
        .with_goal(Goal {
            id: "optimize_workflow".to_string(),
            description: "Optimize data processing workflow".to_string(),
            success_criteria: vec!["Reduce latency by 30%".to_string()],
            constraints: vec!["Budget under $5k".to_string()],
            priority: 8,
        })
        .on_success(WorkflowState::Success)
        .on_error(WorkflowState::Error)
        .build()?;

    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .node(WorkflowState::Start, planner)
        .build()?;

    let result = flow.execute(Context::new()).await?;
    println!("Planning completed: {:?}", result.final_state);
    Ok(())
}
```

## 🏗️ Architecture

```text
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│ pocketflow-core │    │ pocketflow-mcp   │    │ pocketflow-cognitive│
├─────────────────┤    ├──────────────────┤    ├─────────────────────┤
│ • Node trait    │    │ • MCP Client     │    │ • ThinkingNode      │
│ • Context       │    │ • MCP Server     │    │ • PlanningNode      │
│ • FlowState     │    │ • Registry       │    │ • Memory Systems    │
│ • SimpleFlow    │    │ • Context Ext    │    │ • Cognitive Traits  │
│ • AdvancedFlow  │    │ • MCP Nodes      │    │ • Goal-Oriented     │
└─────────────────┘    └──────────────────┘    └─────────────────────┘
         │                        │                          │
         └───────┬────────────────┼─────────────────────────┘
                 │                │
    ┌─────────────────────────┐   │    ┌─────────────────────┐
    │  pocketflow-agent       │   │    │  pocketflow-tools   │
    ├─────────────────────────┤   │    ├─────────────────────┤
    │ • AgentNode             │   │    │ • Tool trait        │
    │ • GenAI Integration     │   │    │ • ToolRegistry      │
    │ • Multi-Agent Support   │   │    │ • Parameter Schema  │
    │ • Execution History     │   │    │ • Validation        │
    │ • Streaming             │   │    │ • Built-in Tools    │
    └─────────────────────────┘   │    └─────────────────────┘
                 │                │              │
                 └────────────────┼──────────────┘
                                  │
                ┌─────────────────────────┐
                │    Your Application     │
                │                         │
                │ • Custom Nodes          │
                │ • Workflow Logic        │
                │ • AI Agents             │
                │ • Cognitive Planning    │
                │ • Tool Integration      │
                │ • MCP Services          │
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
cargo run --example simple_agent_demo --package pocketflow-agent
cargo run --example thinking_workflow --package pocketflow-cognitive
cargo run --example simple_mcp_demo --package pocketflow-mcp
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
- ✅ HTTP transport with authentication
- ⏳ WebSocket transport (planned)
- ⏳ Prompt templates (planned)

### Cognitive Extensions Features

- ✅ Chain-of-thought reasoning
- ✅ Goal-oriented planning
- ✅ Hierarchical planning
- ✅ Multi-layered memory systems
- ✅ Reflection and explanation nodes
- ✅ MCP integration for AI services
- ⏳ Adaptive planning (in development)
- ⏳ Learning capabilities (planned)

### AI Agent Features

- ✅ GenAI integration (OpenAI, etc.)
- ✅ AgentNode for workflow integration
- ✅ Multi-step execution with history
- ✅ Tool registry integration
- ✅ Streaming support
- ✅ Multiple agent coordination
- ⏳ Custom model providers (planned)
- ⏳ Advanced agent orchestration (planned)

### Tool System Features

- ✅ Tool abstraction with JSON schema
- ✅ Parameter validation
- ✅ Tool registry and discovery
- ✅ Built-in utility tools
- ✅ Retry and timeout mechanisms
- ✅ Custom tool development
- ⏳ Tool composition (planned)
- ⏳ Advanced caching (planned)

## 🎯 Use Cases

### Data Processing Pipelines

Use `pocketflow-core` for structured data transformations with state tracking and error handling.

### AI-Powered Workflows  

Combine `pocketflow-agent` with `pocketflow-tools` to build intelligent workflows that can reason, plan, and execute complex tasks using LLMs.

### Cognitive Task Planning

Use `pocketflow-cognitive` for workflows that need planning, reasoning, and memory capabilities for complex problem-solving.

### API Orchestration

Chain multiple service calls with error handling, retry logic, and state management using the core framework.

### Tool Automation

Use `pocketflow-tools` to create standardized, validated tool interfaces for workflow automation.

### AI Agent Ecosystems

Build multi-agent systems using `pocketflow-agent` with coordination, communication, and task delegation.

### MCP Service Integration

Use `pocketflow-mcp` as a protocol for service-to-service communication and external tool integration within workflows.

## 📚 Documentation

- [Core Framework Documentation](./pocketflow-core/README.md)
- [MCP Integration Documentation](./pocketflow-mcp/README.md)
- [Cognitive Extensions Documentation](./pocketflow-cognitive/README.md)
- [AI Agent Framework Documentation](./pocketflow-agent/README.md)
- [Tool System Documentation](./pocketflow-tools/README.md)
- [API Documentation](https://docs.rs/pocketflow-core)
- [Examples Directory](./pocketflow-core/examples/)

## 🤝 Contributing

Contributions are welcome! Please:

1. Check existing issues and PRs
2. Follow the coding conventions
3. Add tests for new features
4. Update documentation as needed

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
