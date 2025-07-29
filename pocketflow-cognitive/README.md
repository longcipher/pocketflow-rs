# PocketFlow Cognitive Extensions

Cognitive extensions for PocketFlow that add thinking, planning, and reasoning capabilities to workflows without modifying the core framework.

## 🧠 Features

- **Thinking Nodes**: Chain-of-thought reasoning, reflection, and explanation generation
- **Planning Nodes**: Goal-oriented planning, hierarchical task decomposition, and adaptive replanning  
- **Memory Systems**: Multi-layered memory including working, episodic, and semantic memory
- **Context Extensions**: Enhanced context management with cognitive capabilities
- **MCP Integration**: Seamless integration with Model Context Protocol for AI services
- **Non-intrusive Design**: Extends existing traits without modifying pocketflow-core

## 🚀 Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
pocketflow-core = "0.1.0"
pocketflow-cognitive = "0.1.0"
```

### Basic Thinking Workflow

```rust
use pocketflow_core::prelude::*;
use pocketflow_cognitive::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MyState {
    Start, Thinking, Success, Error
}

impl FlowState for MyState {
    fn is_terminal(&self) -> bool {
        matches!(self, MyState::Success | MyState::Error)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create MCP client for AI services
    let mcp_client = create_mcp_client().await?;
    
    // Create a thinking node
    let thinking_node = ChainOfThoughtNode::builder()
        .name("reasoner")
        .with_mcp_client(mcp_client)
        .with_reasoning_strategy(ReasoningStrategy::StepByStep)
        .on_success(MyState::Success)
        .on_error(MyState::Error)
        .build()?;
    
    // Build workflow
    let flow = SimpleFlow::builder()
        .initial_state(MyState::Start)
        .add_node(MyState::Start, passthrough("start", MyState::Thinking))
        .add_node(MyState::Thinking, thinking_node)
        .build()?;
    
    // Set up context with a problem to solve
    let mut context = Context::new();
    context.set("problem", "How to optimize database performance")?;
    context.set_cognitive_memory(CognitiveMemory::new())?;
    
    // Execute workflow
    let result = flow.execute(context).await?;
    println!("Reasoning complete: {:?}", result.final_state);
    
    Ok(())
}
```

### Goal-Oriented Planning

```rust
use pocketflow_cognitive::prelude::*;

// Create a planning node
let planning_node = GoalOrientedPlanningNode::builder()
    .name("planner")
    .with_mcp_client(mcp_client)
    .with_planning_strategy(PlanningStrategy::Hierarchical)
    .with_goal(Goal {
        id: "optimize_system".to_string(),
        description: "Optimize system performance".to_string(),
        success_criteria: vec!["Reduce latency by 50%".to_string()],
        constraints: vec!["Budget under $10k".to_string()],
        priority: 8,
    })
    .on_success(MyState::Success)
    .on_error(MyState::Error)
    .build()?;
```

### Extending Existing Nodes with Cognitive Capabilities

```rust
use pocketflow_cognitive::traits::CognitiveNodeExt;

// Wrap any existing node with cognitive capabilities
let enhanced_node = my_existing_node
    .with_cognitive(thinking_implementation);
```

## 🏗️ Architecture

### Core Components

- **ThinkingNode**: Multi-step reasoning with chain-of-thought capability
- **PlanningNode**: Goal decomposition and execution planning
- **CognitiveMemory**: Multi-layered memory system (working, episodic, semantic)
- **CognitiveContextExt**: Extensions for Context with memory management
- **CognitiveWrapper**: Add cognitive capabilities to any existing node

### Memory Layers

1. **Working Memory**: Short-term reasoning context and active thoughts
2. **Episodic Memory**: Execution experiences and outcomes for learning
3. **Semantic Memory**: Domain knowledge, concepts, and learned patterns

### Integration with MCP

The cognitive nodes leverage Model Context Protocol (MCP) to call:
- LLM services for reasoning and planning
- Vector databases for memory retrieval
- External planning algorithms
- Validation and critique services

## 📚 Examples

### Chain-of-Thought Reasoning

```rust
let thinking_node = ChainOfThoughtNode::builder()
    .with_reasoning_strategy(ReasoningStrategy::StepByStep)
    .with_config(ThinkingConfig {
        max_reasoning_steps: 10,
        confidence_threshold: 0.8,
        enable_reflection: true,
        enable_explanation: true,
        ..Default::default()
    })
    .build()?;
```

### Hierarchical Planning

```rust
let planning_node = GoalOrientedPlanningNode::builder()
    .with_planning_strategy(PlanningStrategy::Hierarchical)
    .with_config(PlanningConfig {
        max_plan_depth: 5,
        max_steps_per_plan: 20,
        enable_risk_assessment: true,
        enable_resource_estimation: true,
        ..Default::default()
    })
    .build()?;
```

### Memory Management

```rust
// Initialize cognitive memory
let mut context = Context::new();
context.set_cognitive_memory(CognitiveMemory::new())?;

// Add thoughts during reasoning
context.add_thought("Analyzing the problem structure".to_string())?;
context.set_reasoning_focus("Database optimization".to_string())?;

// Retrieve memory later
let recent_thoughts = context.get_recent_thoughts()?;
let current_focus = context.get_reasoning_focus()?;
```

## 🔧 Configuration

### Reasoning Strategies

- `StepByStep`: Logical step-by-step reasoning
- `Decomposition`: Break problems into sub-problems  
- `Analogical`: Reason using similar cases
- `Critical`: Critical thinking with explicit evaluation
- `Creative`: Creative thinking for novel solutions

### Planning Strategies

- `Sequential`: Linear step-by-step planning
- `Hierarchical`: Hierarchical task decomposition
- `Parallel`: Parallel execution planning
- `Adaptive`: Adaptive planning with feedback loops
- `BackwardChaining`: Goal-oriented backward planning

## 🤝 Integration with Existing Code

This crate is designed to extend existing PocketFlow workflows without breaking changes:

```rust
// Your existing workflow
let existing_flow = SimpleFlow::builder()
    .add_node(MyState::Process, my_processing_node)
    .build()?;

// Enhanced with cognitive capabilities
let cognitive_flow = SimpleFlow::builder()
    .add_node(MyState::Think, thinking_node)
    .add_node(MyState::Plan, planning_node)
    .add_node(MyState::Process, my_processing_node) // Unchanged!
    .build()?;
```

## 📈 Performance Considerations

- Cognitive operations involve LLM calls which add latency
- Memory systems are bounded to prevent unlimited growth
- Async design allows for concurrent cognitive processing
- MCP integration supports connection pooling and caching

## 🔍 Debugging and Observability

```rust
// Access reasoning traces
let reasoning_trace = context.get_reasoning_trace()?;

// Examine memory state
let memory = context.get_cognitive_memory()?;
println!("Working memory: {:?}", memory.working_memory);
println!("Recent episodes: {:?}", memory.episodic_memory.get_recent_episodes(5));

// Get explanations for decisions
let explanation = thinking_node.explain(&context, &decision).await?;
```

## 🧪 Testing

Run the test suite:

```bash
cargo test --package pocketflow-cognitive
```

Run examples:

```bash
cargo run --example thinking_workflow --package pocketflow-cognitive
```

## 🔮 Future Enhancements

- Multi-agent coordination and workflow collaboration
- Advanced learning algorithms for pattern recognition
- Integration with more AI services and planning algorithms
- Visual reasoning trace and plan visualization
- Distributed cognitive processing

## 📄 License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
