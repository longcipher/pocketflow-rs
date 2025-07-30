use std::sync::Arc;

use pocketflow_agent::{MultiAgentResult, prelude::*};
use pocketflow_core::prelude::*;
use pocketflow_tools::prelude::*;
use tokio_stream::StreamExt;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Advanced Agent Demo");

    // Create tool registry
    let mut tool_registry = ToolRegistry::new();

    // Register some basic tools
    tool_registry
        .register_tool(Box::new(BasicTool::new(
            "web_search",
            "Search the web for information",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
            |params| {
                Box::pin(async move {
                    let query = params
                        .get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default query");
                    Ok(serde_json::json!({
                        "results": format!("Search results for: {}", query),
                        "count": 5
                    }))
                })
            },
        )))
        .await?;

    tool_registry
        .register_tool(Box::new(BasicTool::new(
            "calculate",
            "Perform mathematical calculations",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate"
                    }
                },
                "required": ["expression"]
            }),
            |params| {
                Box::pin(async move {
                    let expr = params
                        .get("expression")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1+1");
                    Ok(serde_json::json!({
                        "result": format!("Result of {}: 42", expr),
                        "expression": expr
                    }))
                })
            },
        )))
        .await?;

    let tool_registry = Arc::new(tool_registry);

    // Demo 1: Basic Streaming Agent
    info!("\n📡 Demo 1: Streaming Agent Execution");
    streaming_agent_demo(tool_registry.clone()).await?;

    // Demo 2: Multi-Agent Coordination
    info!("\n🤝 Demo 2: Multi-Agent Coordination");
    demo_multi_agent_coordination(tool_registry.clone()).await?;

    // Demo 3: Advanced Multi-Agent with Streaming
    info!("\n🎯 Demo 3: Streaming Multi-Agent Workflow");
    demo_streaming_multi_agent(tool_registry.clone()).await?;

    // Demo 4: Integration with PocketFlow Core
    info!("\n🔗 Demo 4: PocketFlow Core Integration");
    demo_pocketflow_integration(tool_registry).await?;

    info!("✅ All demos completed successfully!");
    Ok(())
}

async fn streaming_agent_demo(
    tool_registry: Arc<ToolRegistry>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    info!("Creating streaming agent...");

    // Create a research agent with streaming capabilities
    let research_agent = Arc::new(
        AgentNodeBuilder::new("researcher", "Advanced research assistant")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt("You are a thorough research assistant. Always search for information and provide detailed analysis.")
            .with_tool_registry(tool_registry)
            .with_max_steps(5)
            .build()
            .await?
    );

    let streaming_node = StreamingAgentNodeBuilder::new()
        .with_agent(research_agent)
        .with_name("streaming_researcher")
        .with_buffer_size(100)
        .enable_step_streaming(true)
        .enable_thinking_streaming(true)
        .enable_tool_streaming(true)
        .build()?;

    // Execute with streaming
    let task = "Research the latest developments in AI agents and summarize the key trends";
    let (stream, handle) = streaming_node.execute_streaming(task).await?;

    info!("📡 Streaming execution started...");

    // Process stream chunks
    let mut chunk_count = 0;
    let mut token_count = 0;
    let mut tool_calls = 0;

    let mut stream = Box::pin(stream);
    while let Some(chunk) = stream.next().await {
        chunk_count += 1;

        match chunk {
            StreamChunk::Token {
                content,
                position,
                is_final,
            } => {
                print!("{}", content);
                if is_final {
                    println!();
                }
                token_count += 1;
            }
            StreamChunk::Step { step, step_index } => {
                info!(
                    "  Step {}: {:?} - {}",
                    step_index,
                    step.step_type,
                    step.output
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or("No output")
                );
            }
            StreamChunk::ToolCall {
                tool_name,
                arguments,
                call_id,
            } => {
                info!("  🔧 Tool Call: {} with args: {}", tool_name, arguments);
                tool_calls += 1;
            }
            StreamChunk::ToolResult {
                call_id,
                result,
                success,
            } => {
                info!("  ✅ Tool Result: {} (success: {})", result, success);
            }
            StreamChunk::Thinking {
                content,
                reasoning_type,
            } => {
                info!("  💭 Thinking ({}): {}", reasoning_type, content);
            }
            StreamChunk::Error { error, recoverable } => {
                warn!("  ❌ Error: {} (recoverable: {})", error, recoverable);
            }
            StreamChunk::End {
                final_result,
                success,
            } => {
                info!("  🏁 Stream ended. Success: {}", success);
                if let Some(result) = final_result {
                    info!("  Final result: {}", result);
                }
                break;
            }
            _ => {}
        }
    }

    info!(
        "📊 Streaming stats: {} chunks, {} tokens, {} tool calls",
        chunk_count, token_count, tool_calls
    );

    Ok(())
}

async fn demo_multi_agent_coordination(
    tool_registry: Arc<ToolRegistry>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    info!("Creating multi-agent team...");

    // Create specialized agents
    let researcher = Arc::new(
        AgentNodeBuilder::new("researcher", "Research specialist")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt(
                "You are a research specialist. Focus on finding accurate information.",
            )
            .with_tool_registry(tool_registry.clone())
            .with_priority(Priority::High)
            .build()
            .await?,
    );

    let analyst = Arc::new(
        AgentNodeBuilder::new("analyst", "Data analyst")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt(
                "You are a data analyst. Focus on analyzing and interpreting information.",
            )
            .with_tool_registry(tool_registry.clone())
            .with_priority(Priority::Normal)
            .build()
            .await?,
    );

    let coordinator = Arc::new(
        AgentNodeBuilder::new("coordinator", "Team coordinator")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt(
                "You are a team coordinator. Delegate tasks and synthesize results.",
            )
            .with_tool_registry(tool_registry.clone())
            .with_priority(Priority::High)
            .build()
            .await?,
    );

    // Test different coordination strategies
    let strategies = vec![
        ("Sequential", CoordinationStrategy::Sequential),
        ("Parallel", CoordinationStrategy::Parallel),
        ("Hierarchical", CoordinationStrategy::Hierarchical),
        ("Voting", CoordinationStrategy::Voting),
        ("Round-robin", CoordinationStrategy::RoundRobin),
    ];

    let task = "Analyze the market trends for AI technology in 2024";

    for (strategy_name, strategy) in strategies {
        info!("\n🔄 Testing {} strategy", strategy_name);

        let multi_agent = if strategy == CoordinationStrategy::Hierarchical {
            MultiAgentNodeBuilder::new(format!("team_{}", strategy_name.to_lowercase()))
                .with_coordinator(coordinator.clone())
                .add_agent("researcher", researcher.clone())
                .add_agent("analyst", analyst.clone())
                .with_strategy(strategy)
                .build()
                .await?
        } else {
            MultiAgentNodeBuilder::new(format!("team_{}", strategy_name.to_lowercase()))
                .add_agent("researcher", researcher.clone())
                .add_agent("analyst", analyst.clone())
                .with_strategy(strategy)
                .build()
                .await?
        };

        let start_time = std::time::Instant::now();
        let result = multi_agent.execute_multi_agent(task).await?;
        let duration = start_time.elapsed();

        info!(
            "  Result: {} (took {:?})",
            if result.success { "Success" } else { "Failed" },
            duration
        );

        if result.success {
            info!("  Final answer: {}", result.final_result);
            info!(
                "  Agents involved: {}",
                result
                    .agent_results
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else if let Some(error) = &result.error {
            warn!("  Error: {}", error);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Ok(())
}

async fn demo_streaming_multi_agent(
    tool_registry: Arc<ToolRegistry>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    info!("Creating streaming multi-agent workflow...");

    // Create agents with streaming capabilities
    let agents = vec![
        AgentNodeBuilder::new("researcher", "Research agent")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt("Research information thoroughly")
            .with_tool_registry(tool_registry.clone())
            .build()
            .await?,
        AgentNodeBuilder::new("writer", "Content writer")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt("Write clear and engaging content")
            .with_tool_registry(tool_registry.clone())
            .build()
            .await?,
    ];

    // Create streaming versions
    let streaming_agents: Vec<_> = agents
        .into_iter()
        .map(|agent| {
            let agent_name = agent.config.name.clone();
            Arc::new(
                StreamingAgentNodeBuilder::new()
                    .with_agent(Arc::new(agent))
                    .with_name(format!("streaming_{}", agent_name))
                    .build()
                    .unwrap(),
            )
        })
        .collect();

    let task = "Create a comprehensive report on renewable energy trends";

    info!("🎯 Executing streaming multi-agent workflow...");

    // Execute agents sequentially with streaming
    let current_input = task.to_string();
    let mut all_results = Vec::new();

    for (i, streaming_agent) in streaming_agents.iter().enumerate() {
        info!("\n  Agent {}: {}", i + 1, streaming_agent.name());

        let (stream, _handle) = streaming_agent.execute_streaming(&current_input).await?;

        // Process stream and collect result
        let result = streaming_agent
            .execute_with_processor(&current_input, |chunk| match chunk {
                StreamChunk::Token { content, .. } => {
                    print!("{}", content);
                    true
                }
                StreamChunk::Step { step, step_index } => {
                    info!("\n    Step {}: {:?}", step_index, step.step_type);
                    true
                }
                StreamChunk::End { .. } => {
                    println!("\n");
                    false
                }
                _ => true,
            })
            .await?;

        all_results.push(result.clone());

        // Use result as input for next agent (simplified for demo)
        if let Some(final_result) = &result.final_result {
            info!(
                "  Agent produced result: {}",
                final_result.chars().take(100).collect::<String>()
            );
        }
    }

    // Summary
    info!("\n📊 Streaming Multi-Agent Summary:");
    for (i, result) in all_results.iter().enumerate() {
        info!(
            "  Agent {}: {} tokens, {} chunks, Success: {}",
            i + 1,
            result.total_tokens,
            result.chunks.len(),
            result.success
        );
    }

    if let Some(final_result) = &all_results.last().unwrap().final_result {
        info!("\n✨ Final collaborative result: {}", final_result);
    }

    Ok(())
}

async fn demo_pocketflow_integration(
    tool_registry: Arc<ToolRegistry>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating PocketFlow Core integration...");

    // Create agent nodes for workflow
    let agent1 = Arc::new(
        AgentNodeBuilder::new("planner", "Planning agent")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt("You are a planning agent. Create detailed plans.")
            .with_tool_registry(tool_registry.clone())
            .build()
            .await?,
    );

    let agent2 = Arc::new(
        AgentNodeBuilder::new("executor", "Execution agent")
            .with_openai_model("gpt-4o-mini")
            .with_system_prompt("You are an execution agent. Execute plans step by step.")
            .with_tool_registry(tool_registry)
            .build()
            .await?,
    );

    // Create multi-agent node
    let multi_agent_node = MultiAgentNodeBuilder::new("workflow_team")
        .add_agent("planner", agent1)
        .add_agent("executor", agent2)
        .with_strategy(CoordinationStrategy::Sequential)
        .build()
        .await?;

    // Create a simple workflow using PocketFlow Core
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum WorkflowState {
        Start,
        Planning,
        Executing,
        Success,
        Error,
    }

    impl FlowState for WorkflowState {
        fn is_terminal(&self) -> bool {
            matches!(self, WorkflowState::Success | WorkflowState::Error)
        }
    }

    // Instead of using SimpleFlow with incompatible types, directly execute the multi-agent node
    let mut context = Context::new();
    context.set(
        "task",
        "Create a plan for building a web application and then execute the first steps",
    )?;

    info!("🔗 Executing multi-agent node directly...");

    let result = multi_agent_node.execute(context).await?;

    info!(
        "  Multi-agent execution completed with state: {:?}",
        result.1
    );

    if let Ok(Some(answer)) = result.0.get_json::<serde_json::Value>("final_answer") {
        info!("  Final answer: {}", answer);
    }

    // Also demonstrate streaming in workflow
    let streaming_node = StreamingAgentNodeBuilder::new()
        .with_agent(Arc::new(
            AgentNodeBuilder::new("summary", "Summary agent")
                .with_openai_model("gpt-4o-mini")
                .with_system_prompt("Provide concise summaries")
                .build()
                .await?,
        ))
        .build()?;

    let mut streaming_context = Context::new();
    streaming_context.set(
        "task",
        "Summarize the benefits of AI agents in workflow automation",
    )?;

    info!("\n📡 Executing streaming node in PocketFlow...");
    let (result_context, result_state) = streaming_node.execute(streaming_context).await?;

    info!("  Streaming node completed with state: {:?}", result_state);

    if let Ok(Some(chunks)) = result_context.get_json::<serde_json::Value>("stream_chunks") {
        info!(
            "  Collected {} stream chunks",
            chunks.as_array().map(|a| a.len()).unwrap_or(0)
        );
    }

    Ok(())
}

// Helper function to simulate a basic tool
struct BasicTool {
    name: String,
    description: String,
    parameters_schema: serde_json::Value,
    handler: Box<
        dyn Fn(
                serde_json::Value,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = std::result::Result<
                                serde_json::Value,
                                Box<dyn std::error::Error + Send + Sync>,
                            >,
                        > + Send,
                >,
            > + Send
            + Sync,
    >,
}

impl BasicTool {
    fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters_schema: serde_json::Value,
        handler: F,
    ) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<
                Output = std::result::Result<
                    serde_json::Value,
                    Box<dyn std::error::Error + Send + Sync>,
                >,
            > + Send
            + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            parameters_schema,
            handler: Box::new(move |params| Box::pin(handler(params))),
        }
    }
}

#[async_trait::async_trait]
impl Tool for BasicTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Custom
    }

    fn parameter_schema(&self) -> serde_json::Value {
        self.parameters_schema.clone()
    }

    async fn execute(
        &self,
        parameters: ToolParameters,
        _context: ToolContext,
    ) -> pocketflow_tools::Result<ToolResult> {
        // Convert ToolParameters to serde_json::Value for the handler
        let json_params = parameters.inner().clone();

        // Call the handler
        let result = (self.handler)(json_params)
            .await
            .map_err(|e| ToolError::execution(format!("Tool execution failed: {}", e)))?;

        // Convert result to ToolResult
        Ok(ToolResult::success(result.to_string()))
    }
}
