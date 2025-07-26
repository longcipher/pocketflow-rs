# PocketFlow-RS

A lightweight, type-safe flow-based programming framework for Rust, built on [dptree](https://docs.rs/dptree). PocketFlow-RS provides a minimalist approach to building workflows and state machines with Rust's type system ensuring correctness at compile time.

## 🌟 Features

- **Type-Safe**: Leverage Rust's type system for compile-time correctness
- **Built on dptree**: Utilizes dptree's powerful dependency injection and handler system
- **Advanced Workflows**: Middleware support, conditional routing, and execution analytics
- **Flow Registry**: Manage and execute multiple named workflows
- **Better Error Handling**: Modern error handling with eyre for improved debugging
- **Lightweight**: Minimal dependencies, no external service integrations in core framework
- **Async-First**: Full async/await support with tokio
- **Flexible Context**: Type-safe shared state management between nodes
- **State Machines**: First-class support for complex state transitions
- **Batch Processing**: Built-in support for parallel batch operations
- **Composable**: Easy to extend and integrate with external services

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
pocketflow-rs = "0.1.0"
```

### Basic Example

```rust
use pocketflow_rs::prelude::*;

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

// Define a processing node
#[derive(Debug)]
struct ProcessNode;

#[async_trait]
impl Node for ProcessNode {
    type State = WorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        // Your processing logic here
        let input: String = context.get_json("input")?.unwrap_or_default();
        
        if input.is_empty() {
            context.set("error", "No input provided")?;
            return Ok((context, WorkflowState::Error));
        }
        
        let processed = format!("Processed: {}", input);
        context.set("result", processed)?;
        
        Ok((context, WorkflowState::Success))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a simple flow
    let flow = pocketflow_rs::flow::SimpleFlow::builder()
        .name("BasicWorkflow")
        .initial_state(WorkflowState::Start)
        .node(WorkflowState::Start, ProcessNode)
        .build()?;
    
    // Create context with input data
    let mut context = Context::new();
    context.set("input", "Hello, PocketFlow!")?;
    
    // Execute the flow
    let result = flow.execute(context).await?;
    
    println!("Final state: {:?}", result.final_state);
    println!("Result: {:?}", result.context.get_json::<String>("result")?);
    
    Ok(())
}
```

## 🏗️ Core Concepts

### Node

A `Node` represents a unit of work in your workflow. It takes a context, performs some operation, and returns an updated context along with the next state.

```rust
#[derive(Debug)]
struct MyNode;

#[async_trait]
impl Node for MyNode {
    type State = MyState;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        // Your logic here
        Ok((context, MyState::Success))
    }
}
```

### Context

A type-safe shared state container that passes data between nodes:

```rust
let mut context = Context::new();

// Type-safe storage
context.insert(42i32)?;
context.insert("hello".to_string())?;

// JSON storage
context.set("key", "value")?;
context.set("data", &my_struct)?;

// Retrieval
let number: Option<&i32> = context.get();
let value: Option<String> = context.get_json("key")?;
```

### State

States control the flow of execution and must implement the `FlowState` trait:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MyState {
    Start,
    Processing,
    Success,
    Error,
}

impl FlowState for MyState {
    fn is_terminal(&self) -> bool {
        matches!(self, MyState::Success | MyState::Error)
    }
    
    fn can_transition_to(&self, target: &Self) -> bool {
        // Define valid transitions
        match (self, target) {
            (MyState::Start, MyState::Processing) => true,
            (MyState::Processing, MyState::Success | MyState::Error) => true,
            _ => false,
        }
    }
}
```

### Flow

Orchestrates the execution of nodes:

```rust
// Simple approach
let flow = SimpleFlow::builder()
    .name("MyWorkflow")
    .initial_state(MyState::Start)
    .node(MyState::Start, my_node)
    .build()?;

// Advanced approach with middleware
let flow = AdvancedFlow::builder()
    .name("AdvancedWorkflow")
    .initial_state(MyState::Start)
    .with_middleware(logging_middleware)
    .when_state(MyState::Start, condition, conditional_node)
    .with_analytics()
    .build()?;
```

## 📚 Examples

The repository includes several examples demonstrating different use cases:

- **[basic.rs](examples/basic.rs)**: Simple workflow with validation
- **[state_machine.rs](examples/state_machine.rs)**: Complex order processing system  
- **[batch_flow.rs](examples/batch_flow.rs)**: Parallel batch processing
- **[advanced_flow.rs](examples/advanced_flow.rs)**: Advanced workflow with middleware, conditional routing, and analytics

Run an example:

```bash
cargo run --example basic
cargo run --example state_machine
cargo run --example batch_flow
cargo run --example advanced_flow
```

## 🔧 Advanced Features

### Middleware System

Add pre-execution hooks and logging:

```rust
let flow = AdvancedFlow::builder()
    .name("WorkflowWithMiddleware")
    .with_logging()  // Built-in logging middleware
    .with_timing()   // Built-in timing middleware
    .middleware(|context, state| {
        println!("Processing state: {:?}", state);
        Ok(())
    })
    .build()?;
```

### Conditional Routing

Route based on context state:

```rust
let flow = AdvancedFlow::builder()
    .when_state(
        OrderState::Received,
        |ctx| ctx.get_json::<f64>("amount")?.unwrap_or(0.0) > 100.0,
        HighValueOrderNode,
    )
    .when_state(
        OrderState::Received,
        |ctx| ctx.get_json::<f64>("amount")?.unwrap_or(0.0) <= 100.0,
        StandardOrderNode,
    )
    .build()?;
```

### Flow Registry

Manage multiple named workflows:

```rust
let mut registry = FlowRegistry::new();
registry.register("order_processing", order_flow);
registry.register("payment_processing", payment_flow);

// Execute by name
let result = registry.execute("order_processing", context).await?;
```

### Flow Analytics

Built-in execution metrics:

```rust
let flow = AdvancedFlow::builder()
    .with_analytics()
    .build()?;

let result = flow.execute(context).await?;

println!("Execution time: {:?}", result.analytics.execution_time);
println!("Steps executed: {}", result.analytics.steps_executed);
```

### Helper Nodes

PocketFlow-RS provides several helper nodes for common patterns:

```rust
use pocketflow_rs::node::helpers;

// Passthrough node - transitions to a specific state
let passthrough = helpers::passthrough("name", MyState::Success);

// Conditional node - chooses state based on predicate
let conditional = helpers::conditional(
    "condition_check",
    |ctx: &Context| ctx.get_json::<bool>("flag").unwrap_or(Some(false)).unwrap(),
    MyState::Success,
    MyState::Error,
);

// Functional node - from async closure
let func_node = helpers::fn_node("processor", |mut ctx: Context| async move {
    ctx.set("processed", true)?;
    Ok((ctx, MyState::Success))
});
```

### Batch Processing

Built-in support for processing collections of items:

```rust
#[derive(Debug)]
struct BatchProcessor;

#[async_trait]
impl Node for BatchProcessor {
    type State = BatchState;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        let items: Vec<DataItem> = context.get_json("batch_data")?.unwrap_or_default();
        
        // Process items in parallel
        let results = process_items_parallel(items).await?;
        
        context.set("results", results)?;
        Ok((context, BatchState::Complete))
    }
}
```

## 🔋 Dependencies

- [dptree](https://crates.io/crates/dptree) 0.5.1 - Enhanced dependency injection and handler system
- [eyre](https://crates.io/crates/eyre) 0.6 - Better error handling and reporting
- [tokio](https://crates.io/crates/tokio) 1.0 - Async runtime and futures
- [serde](https://crates.io/crates/serde) 1.0 - Serialization framework
- [async-trait](https://crates.io/crates/async-trait) 0.1 - Async trait support
- [chrono](https://crates.io/crates/chrono) 0.4 - Date and time handling

## 🎯 Design Philosophy

PocketFlow-RS is designed around several key principles:

1. **Type Safety**: Use Rust's type system to catch errors at compile time
2. **Composability**: Build complex workflows from simple, reusable components
3. **Lightweight**: Minimal dependencies and overhead
4. **Extensibility**: Easy to integrate with external services and libraries
5. **Clarity**: Clear separation of concerns between state, context, and business logic

## 🔄 Compared to Other Solutions

| Feature | PocketFlow-RS | Original PocketFlow | Other Workflow Engines |
|---------|---------------|-------------------|----------------------|
| Type Safety | ✅ Compile-time | ❌ Runtime | ⚠️ Varies |
| Dependencies | 📦 Minimal | 🐍 Python ecosystem | 📚 Heavy |
| Performance | 🚀 Fast (Rust) | 🐌 Slower (Python) | ⚠️ Varies |
| Middleware | ✅ Built-in | ❌ Manual | ⚠️ Varies |
| Analytics | ✅ Built-in | ❌ Manual | ⚠️ Varies |
| Error Handling | ✅ eyre (Rich) | ⚠️ Basic | ⚠️ Varies |
| Conditional Routing | ✅ Native | ❌ Manual | ⚠️ Varies |
| Flow Registry | ✅ Built-in | ❌ Manual | ⚠️ Varies |
| Learning Curve | 📈 Medium | 📉 Low | 📈 High |
| Ecosystem | 🌱 Growing | 🌳 Established | 🌲 Mature |

## 📦 Architecture Overview

```
pocketflow-rs/
├── Core Framework
│   ├── Context: Type-safe shared state management
│   ├── Node: Async execution units with error handling
│   ├── State: Flow state management with validation
│   ├── Flow: Simple and advanced workflow orchestration
│   └── Error: Modern error handling with eyre
├── Advanced Features
│   ├── Middleware: Pre-execution hooks and logging
│   ├── Analytics: Execution metrics and performance tracking
│   ├── Registry: Named flow management
│   └── Conditional: State-based routing
└── Examples
    ├── basic.rs: Simple workflow demonstration
    ├── state_machine.rs: Complex state transitions
    ├── batch_flow.rs: Parallel processing
    └── advanced_flow.rs: Complete order processing system
```

## 🛠️ Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running Examples

```bash
cargo run --example basic
cargo run --example state_machine
cargo run --example batch_flow
cargo run --example advanced_flow
```

## 📋 Version History

### [0.1.0] - Latest

#### Core Features
- Initial release of PocketFlow-RS
- Core workflow engine built on dptree 0.5.1
- Type-safe context management
- State machine support with validation
- Simple and advanced flow execution
- Enhanced error handling with eyre 0.6

#### Advanced Features
- Advanced flow system with middleware support
- Conditional routing based on context state
- Flow analytics and execution metrics
- Flow registry for managing multiple named flows
- Shared flow state management
- Comprehensive real-world examples

#### Technical Improvements
- Migrated from anyhow to eyre for better error reporting
- Enhanced dptree integration with complex handler patterns
- Async-first design with tokio integration
- Lightweight with minimal dependencies
- Production-ready with comprehensive documentation

## 🚧 Roadmap

- [x] **anyhow → eyre**: Replaced anyhow with eyre for better error handling
- [x] **Advanced dptree Integration**: Enhanced dependency injection and handler patterns with middleware support
- [x] **Flow Analytics**: Built-in execution metrics and performance tracking
- [x] **Conditional Routing**: State-based conditional flow execution
- [x] **Middleware System**: Pre-execution hooks and logging capabilities
- [ ] **Flow Builder**: Visual flow builder with drag-and-drop interface
- [ ] **Persistence**: Built-in support for workflow persistence and recovery
- [ ] **Monitoring**: Advanced metrics and tracing integration
- [ ] **Visual Tools**: Workflow visualization and debugging tools
- [ ] **More Examples**: Additional real-world examples and patterns

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the original [PocketFlow](https://github.com/The-Pocket/PocketFlow) Python framework
- Built on the excellent [dptree](https://docs.rs/dptree) library
- Thanks to the Rust community for the amazing ecosystem

## 📞 Support

- 📖 [Documentation](https://docs.rs/pocketflow-rs)
- 🐛 [Issue Tracker](https://github.com/teloxide/pocketflow-rs/issues)
- 💬 [Discussions](https://github.com/teloxide/pocketflow-rs/discussions)

---

Made with ❤️ in Rust
