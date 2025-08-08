use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TState { Start, Done, Error }

impl pocketflow_core::state::FlowState for TState { fn is_terminal(&self) -> bool { matches!(self, TState::Done | TState::Error) } }

struct MockClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        let resp = match name {
            "llm_reasoning" => "Step 1: Understand the task\nConclusion: Complete steps",
            "llm_reflection" => "Proceed",
            "planning_service" => "Step 1: A\nStep 2: B\nStep 3: C",
            _ => "OK",
        };
        Ok(Value::String(resp.to_string()))
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> {
        Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None })
    }
}

#[tokio::test]
async fn iterative_agent_reaches_completion_with_simulated_execution() -> pocketflow_core::error::Result<()> {
    let client = Arc::new(MockClient);
    let agent = IterativeCognitiveAgentNode::builder()
        .with_mcp_client(client)
        .max_iterations(5)
        .target_completion(100.0)
        .simulate_execution(true)
        .on_success(TState::Done)
        .on_error(TState::Error)
        .build()?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(TState::Start)
        .node(TState::Start, agent)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("problem", "Test completion")?;
    let result = flow.execute(ctx).await?;

    assert_eq!(result.final_state, TState::Done);
    let completion: f64 = result.context.get_json("plan_completion")?.unwrap_or(0.0);
    assert!(completion >= 100.0);
    Ok(())
}
