use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum E2State { Start, Exec, Done, Error }

impl pocketflow_core::state::FlowState for E2State { fn is_terminal(&self) -> bool { matches!(self, E2State::Done | E2State::Error) } }

struct FlakyClient { mut_failures: std::sync::Mutex<usize> }

#[async_trait]
impl pocketflow_mcp::client::McpClient for FlakyClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, _name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        let mut left = self.mut_failures.lock().unwrap();
        if *left > 0 {
            *left -= 1;
            return Err(pocketflow_mcp::error::McpError::ToolExecutionFailed { message: "transient".into() });
        }
        Ok(Value::String("ok".into()))
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn retries_then_succeeds() -> pocketflow_core::error::Result<()> {
    let client = Arc::new(FlakyClient { mut_failures: std::sync::Mutex::new(2) });

    let plan = ExecutionPlan {
        id: "plan2".into(),
        goal: Goal { id: "g2".into(), description: "demo".into(), success_criteria: vec![], constraints: vec![], priority: 5 },
    steps: vec![ PlanStep { id: "s1".into(), description: "A".into(), dependencies: vec![], estimated_duration: std::time::Duration::from_secs(1), required_tools: vec![], success_criteria: vec![], enforce_success_criteria: None, max_retries: None, initial_backoff_ms: None, stop_on_error: None } ],
        estimated_duration: std::time::Duration::from_secs(1),
        required_resources: vec![],
        risk_factors: vec![],
    };

    let exec = PlanExecutionNode::builder()
        .with_mcp_client(client)
        .max_retries(3)
        .initial_backoff_ms(1)
        .stop_on_error(true)
        .on_success(E2State::Done)
        .on_error(E2State::Error)
        .build()?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(E2State::Start)
        .node(E2State::Start, exec)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("execution_plan", &plan)?;

    let result = flow.execute(ctx).await?;
    assert_eq!(result.final_state, E2State::Done);

struct SuccessCriteriaClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for SuccessCriteriaClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, _name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        Ok(json!({
            "status": "ok",
            "data": { "count": 3, "items": ["alpha", "beta"] },
            "message": "processed id=42"
        }))
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn enforces_regex_and_json_pointer_criteria() -> pocketflow_core::error::Result<()> {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum S { Start, Done, Err }
    impl pocketflow_core::state::FlowState for S { fn is_terminal(&self) -> bool { matches!(self, S::Done | S::Err) } }

    let client = std::sync::Arc::new(SuccessCriteriaClient);

    let exec = PlanExecutionNode::builder()
        .with_mcp_client(client)
        .on_success(S::Done)
        .on_error(S::Err)
        .enforce_success_criteria(true)
        .build()?;

    let plan = ExecutionPlan {
        id: "p".into(),
        goal: Goal { id: "g".into(), description: "d".into(), success_criteria: vec![], constraints: vec![], priority: 5 },
        steps: vec![ PlanStep {
            id: "s1".into(),
            description: "desc".into(),
            dependencies: vec![],
            estimated_duration: std::time::Duration::from_secs(1),
            required_tools: vec![],
            success_criteria: vec![
                json!({"regex": "id=\\d+"}),
                json!({"json_pointer": "/data/count", "equals": 3}),
                json!({"json_pointer": "/data/items/0", "exists": true})
            ],
            enforce_success_criteria: None,
            max_retries: None,
            initial_backoff_ms: None,
            stop_on_error: None,
        }],
        estimated_duration: std::time::Duration::from_secs(1),
        required_resources: vec![],
        risk_factors: vec![],
    };

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(S::Start)
        .node(S::Start, exec)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("execution_plan", &plan)?;

    let res = flow.execute(ctx).await?;
    assert_eq!(res.final_state, S::Done);
    Ok(())
}

struct JsonClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for JsonClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, _name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        Ok(json!({ "message": "hello 123 world", "items": ["ax", "b-123-y"] }))
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn json_pointer_contains_works_and_step_enforcement_overrides() -> pocketflow_core::error::Result<()> {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum S { Start, Done, Err }
    impl pocketflow_core::state::FlowState for S { fn is_terminal(&self) -> bool { matches!(self, S::Done | S::Err) } }

    let client = std::sync::Arc::new(JsonClient);

    // Builder sets enforcement to false, but step overrides to true
    let exec = PlanExecutionNode::builder()
        .with_mcp_client(client)
        .enforce_success_criteria(false)
        .on_success(S::Done)
        .on_error(S::Err)
        .build()?;

    let plan = ExecutionPlan {
        id: "p".into(),
        goal: Goal { id: "g".into(), description: "d".into(), success_criteria: vec![], constraints: vec![], priority: 5 },
        steps: vec![ PlanStep {
            id: "s1".into(),
            description: "desc".into(),
            dependencies: vec![],
            estimated_duration: std::time::Duration::from_secs(1),
            required_tools: vec![],
            success_criteria: vec![
                json!({"json_pointer": "/message", "contains": "123"}),
                json!({"json_pointer": "/items", "contains": "123"})
            ],
            enforce_success_criteria: Some(true),
            max_retries: None,
            initial_backoff_ms: None,
            stop_on_error: None,
        }],
        estimated_duration: std::time::Duration::from_secs(1),
        required_resources: vec![],
        risk_factors: vec![],
    };

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(S::Start)
        .node(S::Start, exec)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("execution_plan", &plan)?;

    let res = flow.execute(ctx).await?;
    assert_eq!(res.final_state, S::Done);
    Ok(())
}

struct OneFlakyClient { hit: std::sync::Mutex<bool> }

#[async_trait]
impl pocketflow_mcp::client::McpClient for OneFlakyClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, _name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        let mut hit = self.hit.lock().unwrap();
        if !*hit { *hit = true; return Err(pocketflow_mcp::error::McpError::ToolExecutionFailed { message: "once".into() }); }
        Ok(Value::String("ok".into()))
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn per_step_retry_override_applies() -> pocketflow_core::error::Result<()> {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum S { Start, Done, Err }
    impl pocketflow_core::state::FlowState for S { fn is_terminal(&self) -> bool { matches!(self, S::Done | S::Err) } }

    let client = std::sync::Arc::new(OneFlakyClient { hit: std::sync::Mutex::new(false) });

    // Builder sets max_retries to 0, but step overrides to 1
    let exec = PlanExecutionNode::builder()
        .with_mcp_client(client)
        .max_retries(0)
        .initial_backoff_ms(1)
        .on_success(S::Done)
        .on_error(S::Err)
        .build()?;

    let plan = ExecutionPlan {
        id: "p2".into(),
        goal: Goal { id: "g2".into(), description: "d".into(), success_criteria: vec![], constraints: vec![], priority: 5 },
        steps: vec![ PlanStep {
            id: "s1".into(),
            description: "desc".into(),
            dependencies: vec![],
            estimated_duration: std::time::Duration::from_secs(1),
            required_tools: vec![],
            success_criteria: vec![],
            enforce_success_criteria: None,
            max_retries: Some(1),
            initial_backoff_ms: Some(1),
            stop_on_error: None,
        }],
        estimated_duration: std::time::Duration::from_secs(1),
        required_resources: vec![],
        risk_factors: vec![],
    };

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(S::Start)
        .node(S::Start, exec)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("execution_plan", &plan)?;

    let res = flow.execute(ctx).await?;
    assert_eq!(res.final_state, S::Done);
    Ok(())
}
    let completed: Vec<String> = result.context.get_json("completed_steps")?.unwrap_or_default();
    assert_eq!(completed, vec!["s1".to_string()]);
    Ok(())
}
