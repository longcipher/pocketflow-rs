use std::sync::Arc;

use pocketflow_core::{
    node,
    prelude::{Context, FlowState as PFState, SimpleFlow},
};
use pocketflow_tools::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum FlowState {
    Start,
    Search,
    RunPy,
    Done,
    Error,
}

impl FlowState {
    fn success() -> Self {
        FlowState::Done
    }
    fn error() -> Self {
        FlowState::Error
    }
}

impl PFState for FlowState {
    fn is_terminal(&self) -> bool {
        matches!(self, FlowState::Done | FlowState::Error)
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Registry with our new tools
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(WebSearchTool::new()))
        .await?;
    registry
        .register_tool(Box::new(PythonExecutionTool::new()))
        .await?;
    let registry = Arc::new(registry);

    // Build nodes
    let search_node = node::helpers::fn_node("search", {
        let registry = registry.clone();
        move |mut ctx: Context| {
            let registry = registry.clone();
            Box::pin(async move {
                let q: String = ctx
                    .get_json("query")?
                    .unwrap_or_else(|| "pocketflow".to_string());
                let params = serde_json::json!({"query": q, "simulate": true});
                let res = registry
                    .execute_tool("web_search", &params, &ToolContext::new())
                    .await
                    .unwrap();
                ctx.set(
                    "search_result",
                    &serde_json::from_str::<serde_json::Value>(&res.content).unwrap_or_default(),
                )?;
                Ok((ctx, FlowState::RunPy))
            })
        }
    });

    let python_node = node::helpers::fn_node("run_py", {
        let registry = registry.clone();
        move |mut ctx: Context| {
            let registry = registry.clone();
            Box::pin(async move {
                let code = "import json; print(json.dumps({'ok': True}))";
                let params = serde_json::json!({"code": code});
                let res = registry
                    .execute_tool("python_execute", &params, &ToolContext::new())
                    .await
                    .unwrap();
                let val: serde_json::Value = serde_json::from_str(&res.content).unwrap_or_default();
                ctx.set("python_output", &val)?;
                Ok((ctx, FlowState::Done))
            })
        }
    });

    // Build flow
    let flow = SimpleFlow::builder()
        .name("search_and_python")
        .initial_state(FlowState::Search)
        .node(FlowState::Search, search_node)
        .node(FlowState::RunPy, python_node)
        .build()?;

    // Run flow
    let mut ctx = Context::new();
    ctx.set("query", "rust pocketflow")?;
    let result = flow.execute(ctx).await.unwrap();
    println!("Final state: {:?}", result.final_state);
    let search: Option<serde_json::Value> =
        result.context.get_json("search_result").unwrap_or(None);
    let pyout: Option<serde_json::Value> = result.context.get_json("python_output").unwrap_or(None);
    println!(
        "Search present: {} | Python output present: {}",
        search.is_some(),
        pyout.is_some()
    );

    Ok(())
}
