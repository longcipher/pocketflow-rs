# PocketFlow Agent

[![Crates.io](https://img.shields.io/crates/v/pocketflow-agent.svg)](https://crates.io/crates/pocketflow-agent)
[![Documentation](https://docs.rs/pocketflow-agent/badge.svg)](https://docs.rs/pocketflow-agent)
[![License](https://img.shields.io/crates/l/pocketflow-agent.svg)](https://github.com/longcipher/pocketflow-rs/blob/master/LICENSE)

AI Agent framework for PocketFlow with genai integration. Build intelligent workflow nodes that can reason, plan, and execute tasks using large language models.

## Features

- **GenAI Integration**: Built-in support for OpenAI, Anthropic, and other LLM providers via the `genai` crate
- **Agent Nodes**: Pre-built workflow nodes that can process tasks using AI
- **Multi-Agent Systems**: Coordinate multiple agents for complex workflows
- **Streaming Support**: Real-time streaming responses from AI models
- **Tool Integration**: Seamless integration with `pocketflow-tools` for function calling
- **Async/Await**: Full async support for non-blocking AI operations

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
pocketflow-agent = "0.2.0"
pocketflow-core = "0.2.0"
```

## Basic Usage

```rust
use pocketflow_agent::{AgentNode, AgentConfig, ModelConfig, ModelProvider};
use pocketflow_core::{Context, SimpleFlow};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MyState {
    Start,
    Processing,
    Success,
    Error,
}

impl pocketflow_core::FlowState for MyState {
    fn is_terminal(&self) -> bool {
        matches!(self, MyState::Success | MyState::Error)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an agent node
    let agent = AgentNode::new(AgentConfig {
        name: "task_processor".to_string(),
        model_config: ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "gpt-4o-mini".to_string(),
            ..Default::default()
        },
        system_prompt: "You are a helpful task processing agent.".to_string(),
        ..Default::default()
    });

    // Create a simple flow
    let mut flow = SimpleFlow::new(MyState::Start);
    flow.add_node(MyState::Start, agent, MyState::Processing);

    // Execute the workflow
    let mut context = Context::new();
    context.set("task", "Analyze this data and provide insights")?;
    
    let final_state = flow.execute(context).await?;
    println!("Workflow completed with state: {:?}", final_state);

    Ok(())
}
```

## Multi-Agent Example

```rust
use pocketflow_agent::{MultiAgentSystem, AgentConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut system = MultiAgentSystem::new();
    
    // Add specialized agents
    system.add_agent("analyst", AgentConfig {
        name: "data_analyst".to_string(),
        system_prompt: "You are a data analysis expert.".to_string(),
        ..Default::default()
    }).await?;
    
    system.add_agent("writer", AgentConfig {
        name: "report_writer".to_string(),
        system_prompt: "You are a technical writer.".to_string(),
        ..Default::default()
    }).await?;
    
    // Coordinate agents
    let analysis = system.execute("analyst", "Analyze quarterly sales data").await?;
    let report = system.execute("writer", &format!("Write a report based on: {}", analysis)).await?;
    
    println!("Final report: {}", report);
    Ok(())
}
```

## Streaming Support

```rust
use pocketflow_agent::{AgentNode, StreamingConfig};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = AgentNode::new(AgentConfig {
        streaming: Some(StreamingConfig::default()),
        ..Default::default()
    });
    
    let mut stream = agent.stream("Generate a story about AI").await?;
    
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?);
    }
    
    Ok(())
}
```

## Tool Integration

```rust
use pocketflow_agent::AgentNode;
use pocketflow_tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tools = ToolRegistry::new();
    // Register your tools...
    
    let agent = AgentNode::new(AgentConfig::default())
        .with_tools(tools);
    
    // Agent can now call tools during execution
    let result = agent.execute("Calculate the fibonacci sequence up to 10").await?;
    
    Ok(())
}
```

## Configuration

Agents support various configuration options:

```rust
use pocketflow_agent::{AgentConfig, ModelConfig, ModelProvider, ModelParameters};

let config = AgentConfig {
    name: "my_agent".to_string(),
    model_config: ModelConfig {
        provider: ModelProvider::OpenAI,
        model_name: "gpt-4o".to_string(),
        parameters: ModelParameters {
            temperature: 0.7,
            max_tokens: 1000,
            top_p: 0.9,
            ..Default::default()
        },
        ..Default::default()
    },
    system_prompt: "Custom system prompt".to_string(),
    max_history_length: 50,
    timeout_seconds: 30,
    retry_attempts: 3,
    ..Default::default()
};
```

## Cargo Features

- `streaming` (default): Enable streaming response support
- `full`: Enable all features

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
