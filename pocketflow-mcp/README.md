# PocketFlow MCP Integration

Model Context Protocol (MCP) integration for PocketFlow workflows.

## 🚀 Features

- **MCP Client Integration**: Connect to MCP servers and call tools from workflows
- **MCP Server Implementation**: Expose workflow capabilities as MCP tools
- **Workflow Context Extensions**: Seamlessly integrate MCP data with PocketFlow context
- **Registry Management**: Manage multiple MCP clients and servers
- **Type-safe Integration**: Full type safety with PocketFlow's workflow system

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
pocketflow-core = "0.1.0"
pocketflow-mcp = "0.1.0"
```

## 🏃 Quick Start

### Using MCP Client in Workflows

```rust
use pocketflow_core::prelude::*;
use pocketflow_mcp::{prelude::*, client::McpClientNode};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum WorkflowState {
    Start,
    CallingTool,
    Success,
    Error,
}

impl FlowState for WorkflowState {
    fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Success | WorkflowState::Error)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create workflow with MCP integration (HTTP transport)
    let node = McpClientNode::builder("mcp_tool_caller")
        .with_http("http://localhost:8080")
        .tool("my_tool")
        .map_input("param1", "param1")
        .map_input("param2", "param2")
        .output_to("tool_result")
        .on_success(WorkflowState::Success)
        .on_error(WorkflowState::Error)
        .build()?;

    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .node(WorkflowState::Start, node)
        .build()?;

    let mut context = Context::new();
    context.set("param1", "value1")?;
    context.set("param2", 42)?;

    let result = flow.execute(context).await?;
    println!("Final state: {:?}", result.final_state);
    
    Ok(())
}
```

### Creating MCP Server

```rust
use pocketflow_mcp::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create context with workflow capabilities
    let context = Arc::new(RwLock::new(Context::new()));
    
    // Configure MCP server
    let config = McpServerConfig {
        bind_address: "127.0.0.1:8080".to_string(),
        server_name: "PocketFlow MCP Server".to_string(),
        server_version: "1.0.0".to_string(),
    };
    
    // Create and start server
    let handler = WorkflowMcpHandler::new(context, config)
        .with_default_tools();
    
    let server_node = McpServerNode::new(
        "mcp_server".to_string(),
        handler,
        WorkflowState::ServerRunning,
    );
    
    // Run in workflow
    let mut server_context = Context::new();
    let (_context, state) = server_node.execute(server_context).await?;
    println!("Server state: {:?}", state);
    
    Ok(())
}
```

## 🏗️ Core Components

 
### McpClientNode

Integrates MCP tool calls into PocketFlow workflows:

```rust
let client_node = McpClientNode::new(
    "tool_caller".to_string(),      // Node name
    Arc::new(mcp_client),           // MCP client
    "tool_name".to_string(),        // Tool to call
    WorkflowState::Calling,         // Processing state
    WorkflowState::Success,         // Success state
    WorkflowState::Error,           // Error state
);
```

 
### McpServerNode

Exposes workflow capabilities as MCP server:

```rust
let server_node = McpServerNode::new(
    "mcp_server".to_string(),
    workflow_handler,
    WorkflowState::Running,
);
```

 
### Context Extensions

Seamlessly integrate MCP data with workflow context:

```rust
// Add MCP client to context
context.add_mcp_client("my_client".to_string(), Arc::new(client))?;

// Call MCP tool from context
let result = context.call_mcp_tool(
    "my_client",
    "tool_name", 
    serde_json::json!({"param": "value"})
).await?;
```

 
### Registry Management

Manage multiple MCP connections:

```rust
let registry = McpRegistry::new();

// Register clients
registry.register_client("client1".to_string(), Arc::new(client1)).await?;
registry.register_client("client2".to_string(), Arc::new(client2)).await?;

// Register servers
registry.register_server("server1".to_string(), handler1).await?;

// List all connections
let (clients, servers) = registry.list_all().await;
```

## 🔧 MCP Server Tools

The MCP server provides these built-in tools:

- **get_context**: Retrieve workflow context data
- **set_context**: Update workflow context
- **get_flow_state**: Get current workflow state
- **execute_workflow**: Trigger workflow execution

### Custom Tools

Add custom tools to your MCP server:

```rust
let handler = WorkflowMcpHandler::new(context, config)
    .add_tool(Tool {
        name: "custom_tool".to_string(),
        description: "My custom tool".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        }),
    });
```

## 📋 Examples

Run the examples to see MCP integration patterns:

```bash
# Simple MCP demo
cargo run --example mcp_demo_simple

# Full MCP integration example
cargo run --example mcp_integration

# MCP server example  
cargo run --example simple_mcp_demo
```

## 🔌 Supported MCP Features

- ✅ Tool calling
- ✅ Resource access
- ✅ Server information
- ✅ HTTP transport with authentication
- ✅ Error handling and retries
- ⏳ WebSocket transport (planned)
- ⏳ Prompt templates (planned)

## 🎯 Use Cases

- **AI Agent Workflows**: Integrate LLM tools into structured workflows
- **API Orchestration**: Chain multiple API calls with state management
- **Data Pipeline Integration**: Connect data processing tools via MCP
- **Microservice Communication**: Use MCP as a service mesh protocol
- **Workflow Automation**: Expose workflow capabilities to external systems

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
