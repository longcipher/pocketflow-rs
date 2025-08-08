use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum EState { Start, Exec, Done, Error }

impl pocketflow_core::state::FlowState for EState { fn is_terminal(&self) -> bool { matches!(self, EState::Done | EState::Error) } }

struct MockClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, _name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> { Ok(Value::String("ok".into())) }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn executes_all_steps_and_marks_completed() -> pocketflow_core::error::Result<()> {
    let client = Arc::new(MockClient);

    // Construct a tiny plan in context
    let plan = ExecutionPlan {
        id: "plan1".into(),
        goal: Goal { id: "g1".into(), description: "demo".into(), success_criteria: vec![], constraints: vec![], priority: 5 },
        steps: vec![
            PlanStep { id: "s1".into(), description: "A".into(), dependencies: vec![], estimated_duration: std::time::Duration::from_secs(1), required_tools: vec![], success_criteria: vec![], enforce_success_criteria: None, max_retries: None, initial_backoff_ms: None, stop_on_error: None },
            PlanStep { id: "s2".into(), description: "B".into(), dependencies: vec![], estimated_duration: std::time::Duration::from_secs(1), required_tools: vec![], success_criteria: vec![], enforce_success_criteria: None, max_retries: None, initial_backoff_ms: None, stop_on_error: None },
        ],
        estimated_duration: std::time::Duration::from_secs(2),
        required_resources: vec![],
        risk_factors: vec![],
    };

    let exec = PlanExecutionNode::builder()
        .with_mcp_client(client)
        .on_success(EState::Done)
        .on_error(EState::Error)
        .build()?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(EState::Start)
        .node(EState::Start, exec)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("execution_plan", &plan)?;

    let result = flow.execute(ctx).await?;
    assert_eq!(result.final_state, EState::Done);
    let completed: Vec<String> = result.context.get_json("completed_steps")?.unwrap_or_default();
    assert_eq!(completed.len(), 2);
    Ok(())
}
