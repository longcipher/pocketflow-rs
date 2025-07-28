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
use pocketflow_mcp::prelude::*;

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
    // Create MCP client
    let client = UltraFastMcpClient::new("http://localhost:8080").await?;
    
    // Create workflow with MCP integration
    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .add_node(WorkflowState::Start, McpClientNode::new(
            "mcp_tool_caller".to_string(),
            Arc::new(client),
            "my_tool".to_string(),
            WorkflowState::CallingTool,
            WorkflowState::Success,
            WorkflowState::Error,
        ))
        .build()?;

    let mut context = Context::new();
    context.set("tool_args".to_string(), serde_json::json!({
        "param1": "value1",
        "param2": 42
    }))?;

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
