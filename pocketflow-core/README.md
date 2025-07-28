# PocketFlow Core

A lightweight, type-safe workflow framework built on `dptree` with async/await support.

## 🚀 Features

- **Type-safe workflows**: Leverage Rust's type system for compile-time correctness
- **Async/await support**: Built on tokio for high-performance async execution
- **Flexible node system**: Create custom processing units with the `Node` trait
- **Context management**: Type-safe shared state with JSON and typed storage
- **State machines**: Define complex workflow states with transition validation
- **Error handling**: Rich error types with eyre integration
- **Metrics & tracing**: Optional observability features

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
pocketflow-core = "0.1.0"
```

## 🏃 Quick Start

```rust
use pocketflow_core::prelude::*;

// Define your workflow state
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum WorkflowState {
    Start,
    Processing,
    Success,
    Error,
}

impl FlowState for WorkflowState {
    fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Success | WorkflowState::Error)
    }
}

// Create a processing node
#[derive(Clone)]
struct DataProcessor {
    name: String,
}

#[async_trait]
impl Node for DataProcessor {
    type State = WorkflowState;

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        // Process data
        let input = context.get_json("input").unwrap_or_default();
        let processed = format!("Processed: {}", input);
        context.set("processed_data".to_string(), serde_json::json!(processed))?;
        
        Ok((context, WorkflowState::Processing))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Build and execute workflow
    let flow = SimpleFlow::builder()
        .initial_state(WorkflowState::Start)
        .add_node(WorkflowState::Start, DataProcessor {
            name: "processor".to_string(),
        })
        .build()?;

    let mut context = Context::new();
    context.set("input".to_string(), serde_json::json!("Hello, World!"))?;

    let result = flow.execute(context).await?;
    println!("Final state: {:?}", result.final_state);
    
    Ok(())
}
```

## 🏗️ Core Concepts

### Node
A `Node` represents a unit of work that processes context and produces a state transition:

```rust
#[async_trait]
pub trait Node: Send + Sync + Clone {
    type State: FlowState;

    fn name(&self) -> &str;
    async fn execute(&self, context: Context) -> Result<(Context, Self::State)>;
}
```

### Context
Type-safe shared state between nodes with both JSON and typed storage:

```rust
let mut context = Context::new();

// JSON storage
context.set("key".to_string(), serde_json::json!("value"))?;
let value: String = context.get_json("key").unwrap_or_default();

// Typed storage
context.insert(MyStruct { data: 42 });
let my_struct = context.get::<MyStruct>().cloned();
```

### FlowState
Define workflow states with terminal conditions and optional transition validation:

```rust
impl FlowState for MyState {
    fn is_terminal(&self) -> bool {
        matches!(self, MyState::Success | MyState::Error)
    }

    fn can_transition_to(&self, target: &Self) -> bool {
        // Optional: define valid transitions
        match (self, target) {
            (MyState::Start, MyState::Processing) => true,
            (MyState::Processing, MyState::Success) => true,
            _ => false,
        }
    }
}
```

### Flow Types

#### SimpleFlow
Basic workflow execution:

```rust
let flow = SimpleFlow::builder()
    .initial_state(MyState::Start)
    .add_node(MyState::Start, my_node)
    .build()?;

let result = flow.execute(context).await?;
```

#### AdvancedFlow
Enhanced workflows with middleware, analytics, and registry:

```rust
let flow = AdvancedFlow::builder()
    .initial_state(MyState::Start)
    .add_node(MyState::Start, my_node)
    .add_middleware(|ctx| async move { 
        println!("Before node execution");
        Ok(ctx) 
    })
    .build()?;
```

## 🔧 Helper Nodes

The framework provides several helper nodes for common patterns:

- `PassthroughNode`: Simple state transitions
- `ConditionalNode`: Conditional branching based on context
- `FnNode`: Create nodes from async functions
- `BatchNode`: Process collections of data

## 📋 Examples

Run the examples to see different usage patterns:

```bash
# Basic workflow
cargo run --example basic

# State machine with validation
cargo run --example state_machine

# Batch processing
cargo run --example batch_flow

# Advanced features
cargo run --example advanced_flow
```

## 🎯 Features

### Default Features
- `tracing`: Structured logging support

### Optional Features
- `metrics`: Metrics collection support

Enable features in your `Cargo.toml`:

```toml
[dependencies]
pocketflow-core = { version = "0.1.0", features = ["metrics"] }
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
