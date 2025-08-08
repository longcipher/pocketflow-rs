use std::sync::Arc;

use pocketflow_cognitive::prelude::*;
use pocketflow_core::prelude::*;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum AgentState {
    Start,
    Done,
    Fail,
}

impl FlowState for AgentState {
    fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Done | AgentState::Fail)
    }
}

// A tiny mock MCP client for demo purposes
struct MockClient;

#[async_trait::async_trait]
impl pocketflow_mcp::client::McpClient for MockClient {
    async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> {
        Ok(vec![])
    }

    async fn call_tool(
        &self,
        name: &str,
        _arguments: serde_json::Value,
    ) -> pocketflow_mcp::Result<serde_json::Value> {
        // Provide minimal deterministic text outputs that our parsers expect
        let resp = match name {
            "llm_reasoning" => {
                "Step 1: Analyze the problem\nStep 2: Identify constraints\nConclusion: We should create a simple plan."
            }
            "llm_reflection" => "Looks good; consider validating assumptions.",
            "llm_explanation" => "Chose option due to higher expected value.",
            "planning_service" => {
                "1. Gather requirements\n2. Draft plan\n3. Execute tasks\n4. Validate results"
            }
            "hierarchical_planning_service" => {
                "1. Sub-goal: Collect info\n2. Sub-goal: Implement solution"
            }
            "adaptive_planning_service" => {
                "1. Checkpoint: Validate inputs\n2. Adaptation Point: Choose tools\n3. Execute\n4. Checkpoint: Verify"
            }
            _ => "OK",
        };
        Ok(serde_json::Value::String(resp.to_string()))
    }

    async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> {
        Ok(vec![])
    }

    async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<serde_json::Value> {
        Ok(json!({}))
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
async fn main() -> pocketflow_cognitive::Result<()> {
    let client = Arc::new(MockClient);

    let agent = CognitiveAgentNode::builder()
        .name("tp_agent")
        .with_mcp_client(client)
        .on_success(AgentState::Done)
        .on_error(AgentState::Fail)
        .build()?;

    let flow = SimpleFlow::builder()
        .name("tp_flow")
        .initial_state(AgentState::Start)
        .node(AgentState::Start, agent)
        .build()?;

    let mut ctx = Context::new();
    ctx.set("problem", "Design a plan for task X")?;

    let res = flow.execute(ctx).await?;
    println!("final: {:?}", res.final_state);

    if let Some(plan) = res.context.get_raw("execution_plan") {
        println!("plan: {}", plan);
    }
    if let Some(reasoning) = res.context.get_raw("reasoning_chain") {
        println!("reasoning: {}", reasoning);
    }

    Ok(())
}
