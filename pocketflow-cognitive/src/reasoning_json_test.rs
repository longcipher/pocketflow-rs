use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RState { Start, Reasoned, Error }

impl pocketflow_core::state::FlowState for RState { fn is_terminal(&self) -> bool { matches!(self, RState::Reasoned | RState::Error) } }

struct MockClient;

#[async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> { Ok(vec![]) }
    async fn call_tool(&self, name: &str, _arguments: Value) -> pocketflow_mcp::Result<Value> {
        match name {
            "llm_reasoning" => Ok(json!({
                "steps": [
                    {"thought": "t1", "inference": "i1", "confidence": 0.9},
                    {"thought": "t2"}
                ],
                "conclusion": "done",
                "confidence": 0.85
            })),
            _ => Ok(Value::String("ok".into()))
        }
    }
    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> { Ok(vec![]) }
    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<Value> { Ok(Value::Null) }
    async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> { Ok(pocketflow_mcp::ServerInfo { name: "mock".into(), version: "1.0".into(), description: None, authors: None, homepage: None, license: None, repository: None }) }
}

#[tokio::test]
async fn parses_structured_reasoning_json() -> pocketflow_core::error::Result<()> {
    let client = Arc::new(MockClient);

    let node = ChainOfThoughtNode::builder()
        .with_mcp_client(client)
        .on_success(RState::Reasoned)
        .on_error(RState::Error)
        .build()?;

    let mut ctx = pocketflow_core::context::Context::new();
    ctx.set("problem", "demo problem")?;

    let flow = pocketflow_core::flow::SimpleFlow::builder()
        .initial_state(RState::Start)
        .node(RState::Start, node)
        .build()?;

    let result = flow.execute(ctx).await?;
    assert_eq!(result.final_state, RState::Reasoned);

    let chain: crate::traits::ReasoningChain = result.context.get_json("reasoning_chain")?.unwrap();
    assert_eq!(chain.steps.len(), 2);
    assert_eq!(chain.steps[0].thought, "t1");
    assert_eq!(chain.steps[0].inference, "i1");
    assert_eq!(chain.conclusion, "done");
    assert!(chain.confidence > 0.8);
    Ok(())
}
