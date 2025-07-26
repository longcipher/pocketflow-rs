# PocketFlow-RS AI Coding Agent Instructions

## Core Architecture

PocketFlow-RS is a type-safe workflow framework built on dptree with async/await support. Key components:

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

### Error Handling
- Use `FlowError` types with eyre integration for rich error messages
- Create specific error constructors: `FlowError::context()`, `FlowError::construction()`
- Propagate errors with `?` operator throughout async chains

### Context Management
- Prefer JSON storage for serializable data that flows between nodes
- Use typed storage for complex objects and temporary state
- Always handle missing context data gracefully with `.unwrap_or_default()`

## Build & Test Commands

Use `just` for development tasks:
- `just format` - Format code with taplo + cargo fmt (nightly)
- `just lint` - Check formatting, clippy with strict warnings, cargo machete
- `just test` - Run test suite

For examples: `cargo run --example [basic|state_machine|batch_flow|advanced_flow]`

## Project Structure

- `src/lib.rs` - Main exports and prelude module
- `src/context.rs` - Type-safe context management with JSON/typed storage
- `src/node.rs` - Node trait and helper implementations (FnNode, PassthroughNode, ConditionalNode)
- `src/flow_simple.rs` - Basic workflow execution
- `src/flow_advanced.rs` - Advanced flows with middleware, analytics, registry
- `src/state.rs` - State trait definitions
- `src/error.rs` - Error types with thiserror/eyre integration
- `examples/` - Comprehensive usage examples showing real-world patterns

## Dependencies & Integration

Built on:
- **dptree 0.5.1** - Dependency injection foundation
- **eyre 0.6** - Error handling (not anyhow)
- **tokio** - Async runtime with full features
- **serde/serde_json** - Serialization for context data
- **chrono** - Timestamps and metadata

## Testing Patterns

- Use `#[tokio::test]` for async tests
- Test both success and error paths for nodes
- Verify context data transformations
- Test state transitions and terminal state detection
- Use helper functions from `node::helpers` module for common patterns

## Common Gotchas

- State enums must implement `Clone, Debug, PartialEq, Eq, Hash`
- Context mutations require `&mut` - clone context if needed for parallel operations
- AdvancedFlow middleware runs before each node execution
- FlowRegistry manages flows by string names - ensure unique naming
- Terminal states stop execution - design state machines carefully
