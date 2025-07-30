# PocketFlow-RS AI Coding Agent Instructions

## Multi-Crate Architecture

PocketFlow-RS is a Cargo workspace with five main crates:

- **pocketflow-core**: Type-safe workflow framework built on dptree with async/await support
- **pocketflow-mcp**: Model Context Protocol integration for calling external tools and exposing workflows as MCP servers
- **pocketflow-cognitive**: Cognitive extensions adding thinking, planning, and reasoning capabilities via MCP
- **pocketflow-agent**: AI Agent framework with genai integration for building intelligent workflow nodes
- **pocketflow-tools**: Tool system for workflow automation with JSON schema validation and execution

### Core Components

- **Node**: Async execution units implementing `Node` trait with `execute()` method returning `(Context, State)`
- **Context**: Type-safe shared state with both typed storage (`insert<T>()`, `get<T>()`) and JSON storage (`set()`, `get_json()`)  
- **State**: Enums implementing `FlowState` trait with `is_terminal()` and optional `can_transition_to()` methods
- **Flow**: Two variants - `SimpleFlow` for basic workflows and `AdvancedFlow` with middleware/analytics support

## Development Patterns

### State Machine Design
```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MyState {
    Start, Processing, Success, Error
}

impl FlowState for MyState {
    fn is_terminal(&self) -> bool {
        matches!(self, MyState::Success | MyState::Error)
    }
}
```

### Node Implementation
- Always use `#[async_trait]` for Node implementations
- Return `Result<(Context, Self::State)>` from `execute()`
- Use `context.set()` for JSON data, `context.insert()` for typed data
- Implement `name()` method for debugging/logging
- For agent nodes: prefer `AgentNode` from pocketflow-agent for AI-powered processing
- For tool execution: use `ToolRegistry` from pocketflow-tools for schema validation

### Error Handling
- Use `FlowError` types with eyre integration for rich error messages
- Create specific error constructors: `FlowError::context()`, `FlowError::construction()`
- Propagate errors with `?` operator throughout async chains
- MCP errors automatically convert to `FlowError` via `From` implementations
- Agent errors use `AgentError` type with proper error context
- Tool errors use `ToolError` for parameter validation and execution failures

### Context Management
- Prefer JSON storage for serializable data that flows between nodes
- Use typed storage for complex objects and temporary state
- Always handle missing context data gracefully with `.unwrap_or_default()`

## MCP Integration Patterns

### MCP Client Usage
```rust
// Create MCP client node with builder pattern
let client_node = McpClientNode::builder("tool_caller")
    .with_http("http://localhost:8080")
    .tool("summarize_text")
    .on_success(MyState::Success)
    .on_error(MyState::Error)
    .build()?;
```

### MCP Server Exposure
```rust
let server_config = McpServerConfig::default();
let server_node = McpServerNode::new("mcp_server".to_string(), handler, MyState::Running);
```

### Context Extensions
- Use `McpContextExt` trait to add MCP capabilities to Context
- `context.add_mcp_client()` for registering clients
- `context.call_mcp_tool()` for direct tool calls

## Cognitive Extensions Patterns

### Thinking Node Implementation
```rust
let thinking_node = ChainOfThoughtNode::builder()
    .name("reasoner")
    .with_mcp_client(mcp_client)
    .with_reasoning_strategy(ReasoningStrategy::StepByStep)
    .on_success(MyState::Success)
    .on_error(MyState::Error)
    .build()?;
```

### Planning Node with Goal Setting
```rust
let planner = GoalOrientedPlanningNode::builder()
    .with_planning_strategy(PlanningStrategy::Hierarchical)
    .with_goal(Goal {
        id: "optimize_system".to_string(),
        description: "Optimize system performance".to_string(),
        success_criteria: vec!["Reduce latency by 50%".to_string()],
        constraints: vec!["Budget under $10k".to_string()],
        priority: 8,
    })
    .build()?;
```

### Cognitive Wrapper Pattern
- Use `CognitiveNodeExt` trait to wrap existing nodes: `my_node.with_cognitive(cognitive_impl)`
- Cognitive traits extend Node without modification: `CognitiveNode`, `ThinkingNode`, `PlanningNode`
- Memory management via `CognitiveContextExt`: `context.set_cognitive_memory()`, `context.add_thought()`

## AI Agent Integration Patterns

### Agent Node Usage
```rust
let agent = AgentNode::new(AgentConfig {
    name: "task_processor".to_string(),
    model_config: ModelConfig {
        provider: ModelProvider::OpenAI,
        model_name: "gpt-4o-mini".to_string(),
        parameters: ModelParameters::default(),
        api_config: ApiConfig::default(),
    },
    system_prompt: "You are a task processing agent".to_string(),
    ..Default::default()
})
.with_tools(tool_registry);
```

### Multi-Step Agent Execution
- Agents maintain execution history in `Arc<RwLock<Vec<AgentStep>>>`
- Use `agent.step(input).await` for individual interactions
- Access history with `agent.get_history().await`
- Reset state with `agent.reset().await`

## Tool System Patterns

### Tool Registration and Validation
```rust
let mut registry = ToolRegistry::new();
registry.register_tool(Box::new(MyTool::new()))?;

// Tools must implement Tool trait with parameter_schema() and execute()
impl Tool for MyTool {
    fn parameter_schema(&self) -> serde_json::Value {
        ToolParameters::new_schema()
            .add_required("input", "string", "Input text to process")
            .add_optional("options", "object", "Processing options", None)
            .into()
    }
}
```

### Tool Parameter Handling
- Use `ToolParameters::get<T>()` for required parameters
- Use `ToolParameters::get_optional<T>()` for optional parameters
- Tool context includes execution environment, timeouts, and retry configuration
- Return `ToolResult` with success/error status and content type metadata

## Build & Test Commands

Use `just` for development tasks:
- `just format` - Format code with taplo + cargo fmt (nightly)
- `just lint` - Check formatting, clippy with strict warnings, cargo machete
- `just test` - Run test suite

For examples: `cargo run --example [basic|state_machine|batch_flow|advanced_flow] --package pocketflow-core`

## Project Structure

### pocketflow-core/
- `src/lib.rs` - Main exports and prelude module
- `src/context.rs` - Type-safe context management with JSON/typed storage
- `src/node.rs` - Node trait and helper implementations (FnNode, PassthroughNode, ConditionalNode)
- `src/flow_simple.rs` - Basic workflow execution
- `src/flow_advanced.rs` - Advanced flows with middleware, analytics, registry
- `src/state.rs` - State trait definitions
- `src/error.rs` - Error types with thiserror/eyre integration
- `examples/` - Comprehensive usage examples showing real-world patterns

### pocketflow-mcp/
- `src/lib.rs` - MCP integration exports and prelude
- `src/client.rs` - MCP client node implementations and ultrafast-mcp wrappers
- `src/server.rs` - MCP server node and workflow exposure
- `src/context.rs` - Context extensions for MCP integration
- `src/registry.rs` - Multi-client/server management
- `src/error.rs` - MCP-specific error types with conversion to FlowError
- `examples/` - MCP integration patterns

### pocketflow-cognitive/
- `src/lib.rs` - Cognitive extensions exports and prelude
- `src/traits.rs` - Extension traits: CognitiveNode, ThinkingNode, PlanningNode, LearningNode
- `src/thinking/` - Chain-of-thought, reflection, explanation nodes
- `src/planning/` - Goal-oriented, adaptive, hierarchical planning implementations
- `src/memory/` - Multi-layered memory systems (working, episodic, semantic)
- `src/context/` - Cognitive context extensions and memory management
- `examples/` - Cognitive workflow patterns

### pocketflow-agent/
- `src/lib.rs` - Agent framework exports and prelude
- `src/agent_node.rs` - AgentNode implementation with genai integration
- `src/agent_types.rs` - Agent configuration, states, and result types
- `src/error.rs` - Agent-specific error types and conversions
- `examples/` - Agent usage patterns and multi-agent scenarios

### pocketflow-tools/
- `src/lib.rs` - Tool system exports and prelude
- `src/core.rs` - Tool trait, parameters, context, and result types
- `src/registry.rs` - ToolRegistry for managing and executing tools
- `src/validation.rs` - Parameter validation and schema enforcement
- `src/utils.rs` - Utility functions for tool development
- `src/error.rs` - Tool-specific error types with detailed parameter validation

## Dependencies & Integration

Built on:
- **dptree 0.5.1** - Dependency injection foundation
- **eyre 0.6** - Error handling (not anyhow)
- **tokio** - Async runtime with full features
- **serde/serde_json** - Serialization for context data
- **chrono** - Timestamps and metadata
- **ultrafast-mcp** - Model Context Protocol implementation with HTTP transport
- **uuid** - For cognitive plan and task IDs
- **genai** - AI/LLM integration for agent nodes
- **jsonschema** - Tool parameter validation and schema enforcement

## Testing Patterns

- Use `#[tokio::test]` for async tests
- Test both success and error paths for nodes
- Verify context data transformations
- Test state transitions and terminal state detection
- Use helper functions from `node::helpers` module for common patterns
- For MCP: Test tool calls, server responses, and registry operations
- For Cognitive: Test reasoning chains, planning outputs, memory persistence
- For Agents: Test step execution, history management, and error handling
- For Tools: Test parameter validation, execution results, and registry operations

## Common Gotchas

- State enums must implement `Clone, Debug, PartialEq, Eq, Hash`
- Context mutations require `&mut` - clone context if needed for parallel operations
- AdvancedFlow middleware runs before each node execution
- FlowRegistry manages flows by string names - ensure unique naming
- Terminal states stop execution - design state machines carefully
- MCP client connections are async - use `Arc<UltraFastMcpClient>` for sharing
- Module structure: pocketflow-mcp removed the extra `mcp/` subdirectory layer - import directly from crate root
- Cognitive nodes require MCP clients for AI service calls - always provide working client in builders
- Recursive async functions in hierarchical planning require `Box::pin()` - use the pattern in `hierarchical.rs`
- Planning strategies affect node behavior - match strategy to use case (Sequential, Hierarchical, Adaptive, etc.)
- Memory systems are bounded to prevent unlimited growth - configure limits appropriately
- Agent nodes use `AgentState` enum for flow control - map to appropriate business states
- Tool parameter schemas use `jsonschema` crate validation - define schemas with `ToolParameters::new_schema()`
- Tool execution context includes retry config, timeouts, and custom metadata
- Agent execution history is thread-safe via `Arc<RwLock<Vec<AgentStep>>>` - always use `.await` for access
