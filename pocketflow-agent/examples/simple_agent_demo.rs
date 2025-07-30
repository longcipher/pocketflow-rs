use std::sync::Arc;

use pocketflow_agent::prelude::*;
use pocketflow_core::prelude::*;
use pocketflow_tools::prelude::*;
use uuid::Uuid;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🤖 PocketFlow AI Agent Framework Demo");
    println!("=====================================\n");

    // 1. Create a simple agent configuration
    let agent_config = AgentConfig {
        id: Uuid::new_v4(),
        name: "demo_agent".to_string(),
        description: "A demonstration AI agent".to_string(),
        role: AgentRole::Independent,
        capabilities: vec![AgentCapability::Basic, AgentCapability::ToolCalling],
        execution_mode: ExecutionMode::Sync,
        priority: Priority::Normal,
        max_steps: 5,
        timeout: None,
        model_config: ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "gpt-4o-mini".to_string(),
            parameters: ModelParameters {
                temperature: 0.7,
                max_tokens: Some(1000),
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop_sequences: vec![],
            },
            api_config: ApiConfig {
                api_key: None,
                base_url: None,
                timeout: std::time::Duration::from_secs(30),
                max_retries: 3,
                retry_delay: std::time::Duration::from_millis(500),
            },
        },
        system_prompt: "You are a helpful AI assistant that can process text and answer questions."
            .to_string(),
        available_tools: vec!["text_processor".to_string()],
        metadata: std::collections::HashMap::new(),
    };

    // 2. Create tool registry with basic tools
    let mut tool_registry = ToolRegistry::new();

    // Add a simple text processing tool
    let text_tool = pocketflow_tools::custom::helpers::uppercase_tool();
    tool_registry
        .register_tool(Box::new(text_tool))
        .await
        .unwrap();

    println!(
        "✅ Created agent '{}' with {} capabilities",
        agent_config.name,
        agent_config.capabilities.len()
    );

    // 3. Create agent node
    let agent_node = AgentNode::new(agent_config.clone()).with_tools(Arc::new(tool_registry));

    println!("✅ Agent node created with tool registry\n");

    // 4. Test direct agent execution
    println!("🔄 Testing direct agent execution...");
    let result = agent_node
        .step("Hello, this is a test message!".to_string())
        .await?;

    println!("📤 Input: Hello, this is a test message!");
    println!(
        "📥 Output: {}",
        result.final_answer.unwrap_or("No response".to_string())
    );
    println!("⏱️  Duration: {:?}", result.total_duration);
    println!("📊 Steps executed: {}\n", result.steps.len());

    // 5. Test agent as a PocketFlow Node
    println!("🔄 Testing agent as PocketFlow Node...");

    let mut context = Context::new();
    context.set("input", "Process this text: artificial intelligence")?;

    let (result_context, final_state) = agent_node.execute(context).await?;

    println!("📤 Input: Process this text: artificial intelligence");

    if let Ok(Some(agent_result)) = result_context.get_json::<AgentResult>("agent_result") {
        println!(
            "📥 Output: {}",
            agent_result
                .final_answer
                .unwrap_or("No response".to_string())
        );
        println!("✅ Final state: {:?}", final_state);
    } else {
        println!("❌ No agent result found in context");
    }

    // 6. Show execution history
    println!("\n📋 Execution History:");
    let history = agent_node.get_history().await;
    for (i, step) in history.iter().enumerate() {
        println!(
            "  Step {}: {:?} -> {:?} ({}ms)",
            i + 1,
            step.step_type,
            step.output
                .as_ref()
                .map(|v| v.as_str().unwrap_or("complex_data"))
                .unwrap_or("no_output"),
            step.duration.map(|d| d.as_millis()).unwrap_or(0)
        );
    }

    // 7. Demonstrate agent in workflow context
    println!("\n🔄 Testing agent in workflow context...");

    // Create initial context
    let mut workflow_context = Context::new();
    workflow_context.set(
        "input",
        "Analyze this: machine learning is transforming technology",
    )?;

    // Execute the agent node directly (as it implements the Node trait)
    let (final_context, final_state) = agent_node.execute(workflow_context).await?;

    println!("🎯 Workflow completed successfully!");
    println!("📊 Final state: {:?}", final_state);
    if let Ok(Some(final_answer)) = final_context.get_json::<AgentResult>("agent_result") {
        println!(
            "📊 Final workflow result: {}",
            final_answer
                .final_answer
                .unwrap_or("No final answer".to_string())
        );
    }

    println!("\n🎉 Demo completed successfully!");
    println!("\nKey Features Demonstrated:");
    println!("✅ Agent configuration and creation");
    println!("✅ Tool registry integration");
    println!("✅ Direct agent execution");
    println!("✅ PocketFlow Node integration");
    println!("✅ Execution history tracking");
    println!("✅ Simple workflow integration");

    Ok(())
}
