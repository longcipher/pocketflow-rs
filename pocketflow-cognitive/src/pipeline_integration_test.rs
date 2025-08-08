use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PState { Start, Exec, Done, Error }

impl pocketflow_core::state::FlowState for PState { fn is_terminal(&self) -> bool { matches!(self, PState::Done | PState::Error) } }

struct MockClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        let resp = match name {
            "llm_reasoning" => "Step 1: Understand\nConclusion: Execute plan",
            "llm_reflection" => "continue",
            "planning_service" => "Step 1: X\nStep 2: Y",
            "execute_step" => "ok",
            _ => "ok",
        };
        Ok(Value::String(resp.to_string()))
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn end_to_end_pipeline_completes() -> pocketflow_core::error::Result<()> {
    let client = Arc::new(MockClient);

    let agent = CognitiveAgentNode::builder()
        .with_mcp_client(client.clone())
        .on_success(PState::Exec)
        .on_error(PState::Error)
        .build()?;

    let executor = PlanExecutionNode::builder()
        .with_mcp_client(client)
        .tool_name("execute_step")
        .on_success(PState::Done)
        .on_error(PState::Error)
        .build()?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(PState::Start)
        .node(PState::Start, agent)
        .node(PState::Exec, executor)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("problem", "Deliver")?;
    let result = flow.execute(ctx).await?;

    assert_eq!(result.final_state, PState::Done);
    let completed: Vec<String> = result.context.get_json("completed_steps")?.unwrap_or_default();
    assert!(!completed.is_empty());
    Ok(())
}
