use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PState { Start, Planned, Error }

impl pocketflow_core::state::FlowState for PState { fn is_terminal(&self) -> bool { matches!(self, PState::Planned | PState::Error) } }

struct MockClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        match name {
            "planning_service" => Ok(json!({
                "id": "plan-json",
                "steps": [
                    {"id": "a", "description": "first", "estimated_duration_seconds": 1, "required_tools": ["t1"]},
                    {"description": "second"}
                ],
                "estimated_duration_seconds": 2,
                "required_resources": ["cpu"],
                "risk_factors": ["latency"]
            })),
            _ => Ok(Value::String("ok".into()))
        }
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn parses_structured_planning_json() -> pocketflow_core::error::Result<()> {
    let client = Arc::new(MockClient);

    let node = GoalOrientedPlanningNode::builder()
        .with_mcp_client(client)
        .on_success(PState::Planned)
        .on_error(PState::Error)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("goal", Goal { id: "g1".into(), description: "desc".into(), success_criteria: vec![], constraints: vec![], priority: 5 })?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(PState::Start)
        .node(PState::Start, node)
        .build()?;

    let result = flow.execute(ctx).await?;
    assert_eq!(result.final_state, PState::Planned);

    let plan: ExecutionPlan = result.context.get_json("execution_plan")?.unwrap();
    assert_eq!(plan.id, "plan-json");
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].id, "a");
    assert_eq!(plan.steps[0].required_tools, vec!["t1".to_string()]);
    assert!(plan.estimated_duration.as_secs() >= 2);
    assert!(plan.required_resources.contains(&"cpu".to_string()));
    assert!(plan.risk_factors.contains(&"latency".to_string()));
    Ok(())
}

#[tokio::test]
async fn rejects_invalid_planning_json_without_steps() -> pocketflow_core::error::Result<()> {
    struct BadClient;
    #[async_trait]
    impl pocketflow_mcp::client::McpClient for BadClient {
        async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
        async fn call_tool(&self, name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
            match name {
                // Missing required 'steps' field per schema
                "planning_service" => Ok(json!({ "id": "plan-bad" })),
                _ => Ok(Value::String("ok".into()))
            }
        }
        async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
        async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
        async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
    }

    let client = Arc::new(BadClient);
    let node = GoalOrientedPlanningNode::builder()
        .with_mcp_client(client)
        .on_success(PState::Planned)
        .on_error(PState::Error)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("goal", Goal { id: "g1".into(), description: "desc".into(), success_criteria: vec![], constraints: vec![], priority: 5 })?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(PState::Start)
        .node(PState::Start, node)
        .build()?;

    let result = flow.execute(ctx).await?;
    assert_eq!(result.final_state, PState::Error);
    let err: Option<String> = result.context.get_json("planning_error")?;
    assert!(err.unwrap_or_default().contains("Planning JSON invalid"));
    Ok(())
}
