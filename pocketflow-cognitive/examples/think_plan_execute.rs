use std::sync::Arc;

use async_trait::async_trait;
use pocketflow_cognitive::prelude::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum State {
    Start,
    Exec,
    Done,
    Error,
}

impl pocketflow_core::state::FlowState for State {
    fn is_terminal(&self) -> bool {
        matches!(self, State::Done | State::Error)
    }
}

struct MockClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> {
        Ok(vec![])
    }
    async fn call_tool(&self, name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        let resp = match name {
            "llm_reasoning" => "Step 1: Understand\nConclusion: Execute a simple 2-step plan",
            "llm_reflection" => "Stable; continue",
            "planning_service" => "Step 1: Prepare\nStep 2: Finish",
            "execute_step" => "ok",
            _ => "ok",
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

    // Step 1: thinking + planning
    let agent = CognitiveAgentNode::builder()
        .name("agent")
        .with_mcp_client(client.clone())
        .on_success(State::Exec)
        .on_error(State::Error)
        .build()?;

    // Step 2: execute plan
    let executor = PlanExecutionNode::builder()
        .name("executor")
        .with_mcp_client(client)
        .tool_name("execute_step")
        .on_success(State::Done)
        .on_error(State::Error)
        .build()?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(State::Start)
        .node(State::Start, agent)
        .node(State::Exec, executor)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("problem", "Ship an MVP")?;

    let result = flow.execute(ctx).await?;
    println!("Final state: {:?}", result.final_state);
    let completed: Vec<String> = result
        .context
        .get_json("completed_steps")?
        .unwrap_or_default();
    println!("Completed steps: {}", completed.len());
    Ok(())
}
