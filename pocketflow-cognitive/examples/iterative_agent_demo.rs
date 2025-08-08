use std::sync::Arc;

use async_trait::async_trait;
use pocketflow_cognitive::prelude::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DemoState {
    Start,
    Running,
    Done,
    Error,
}

impl pocketflow_core::state::FlowState for DemoState {
    fn is_terminal(&self) -> bool {
        matches!(self, DemoState::Done | DemoState::Error)
    }
}

// Simple deterministic mock MCP client
struct MockClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> {
        Ok(vec![])
    }
    async fn call_tool(&self, name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        let resp = match name {
            "llm_reasoning" => {
                "Step 1: Understand the task\nStep 2: Outline a plan\nConclusion: Create a simple plan and execute steps"
            }
            "llm_reflection" => "Looks good; iterate and complete remaining steps.",
            "planning_service" => "Step 1: Do thing A\nStep 2: Do thing B\nStep 3: Do thing C",
            _ => "OK",
        };
        Ok(Value::String(resp.to_string()))
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> {
        Ok(vec![])
    }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> {
        Ok(Value::Null)
    }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> {
        Ok(pocketflow_mcp::ServerInfo {
            name: "mock".into(),
            version: "1.0".into(),
            description: None,
            authors: None,
            homepage: None,
            license: None,
            repository: None,
        })
    }
}

#[tokio::main]
async fn main() -> pocketflow_core::error::Result<()> {
    let client = Arc::new(MockClient);

    let agent = IterativeCognitiveAgentNode::builder()
        .name("iter_agent")
        .with_mcp_client(client)
        .max_iterations(5)
        .target_completion(100.0)
        .simulate_execution(true)
        .on_success(DemoState::Done)
        .on_error(DemoState::Error)
        .build()?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(DemoState::Start)
        .node(DemoState::Start, agent)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("problem", "Finish the demo task")?;

    let result = flow.execute(ctx).await?;
    println!("Final state: {:?}", result.final_state);
    let completion: f64 = result.context.get_json("plan_completion")?.unwrap_or(0.0);
    println!("Completion: {}", completion);
    Ok(())
}
