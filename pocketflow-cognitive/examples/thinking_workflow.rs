//! Example demonstrating thinking and reasoning capabilities.
//!
//! This example shows how to use ChainOfThoughtNode and other thinking
//! capabilities in a workflow without modifying pocketflow-core.

use std::sync::Arc;

use pocketflow_cognitive::{planning::PlanningStrategy, prelude::*};
use pocketflow_core::prelude::*;
use pocketflow_mcp::client::McpClient;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ProblemSolvingState {
    Start,
    Thinking,
    Planning,
    Executing,
    Reflecting,
    Success,
    Error,
}

impl FlowState for ProblemSolvingState {
    fn is_terminal(&self) -> bool {
        matches!(
            self,
            ProblemSolvingState::Success | ProblemSolvingState::Error
        )
    }

    fn can_transition_to(&self, target: &Self) -> bool {
        use ProblemSolvingState::*;
        match (self, target) {
            (Start, Thinking) => true,
            (Thinking, Planning) => true,
            (Planning, Executing) => true,
            (Executing, Reflecting) => true,
            (Reflecting, Success) => true,
            (_, Error) => true, // Can always transition to error
            _ => false,
        }
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Starting Cognitive Workflow Example");

    // Create MCP client for AI services (this would connect to actual services)
    let mcp_client = create_mock_mcp_client().await?;

    // Create thinking node
    let thinking_node = ChainOfThoughtNode::builder()
        .name("problem_reasoner")
        .with_mcp_client(mcp_client.clone())
        .with_reasoning_strategy(ReasoningStrategy::StepByStep)
        .on_success(ProblemSolvingState::Planning)
        .on_error(ProblemSolvingState::Error)
        .build()?;

    // Create planning node
    let planning_node = GoalOrientedPlanningNode::builder()
        .name("solution_planner")
        .with_mcp_client(mcp_client.clone())
        .with_planning_strategy(PlanningStrategy::Hierarchical)
        .on_success(ProblemSolvingState::Executing)
        .on_error(ProblemSolvingState::Error)
        .build()?;

    // Create execution node (simple passthrough for demo)
    let execution_node =
        pocketflow_core::node::helpers::fn_node("executor", |mut ctx: Context| async move {
            println!("📋 Executing planned actions...");

            // Simulate execution
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            ctx.set("execution_result", "Actions executed successfully")?;

            Ok((ctx, ProblemSolvingState::Reflecting))
        });

    // Create reflection node
    let reflection_node =
        pocketflow_core::node::helpers::fn_node("reflector", |mut ctx: Context| async move {
            println!("🤔 Reflecting on the process...");

            // Add reflection thoughts
            ctx.add_thought("The reasoning process was systematic".to_string())?;
            ctx.add_thought("The plan was well-structured".to_string())?;
            ctx.add_thought("Execution went smoothly".to_string())?;

            ctx.set("reflection_complete", true)?;
            Ok((ctx, ProblemSolvingState::Success))
        });

    // Build the cognitive workflow
    let flow = SimpleFlow::builder()
        .name("cognitive_problem_solver")
        .initial_state(ProblemSolvingState::Start)
        .node(ProblemSolvingState::Start, thinking_node)
        .node(ProblemSolvingState::Planning, planning_node)
        .node(ProblemSolvingState::Executing, execution_node)
        .build()?;

    // Set up the problem context
    let mut context = Context::new();
    context.set(
        "problem",
        "How to optimize a complex software system for better performance",
    )?;

    // Add a goal for planning
    let goal = Goal {
        id: "optimize_system".to_string(),
        description: "Optimize software system performance".to_string(),
        success_criteria: vec![
            "Reduce response time by 50%".to_string(),
            "Maintain system stability".to_string(),
            "Keep resource usage under control".to_string(),
        ],
        constraints: vec![
            "Cannot change core architecture".to_string(),
            "Must maintain backward compatibility".to_string(),
        ],
        priority: 8,
    };
    context.set("goal", &goal)?;

    // Initialize cognitive memory
    context.set_cognitive_memory(CognitiveMemory::new())?;

    println!("🚀 Executing cognitive workflow...");

    // Execute the workflow
    let result = flow.execute(context).await?;

    // Display results
    println!("\n✅ Workflow completed!");
    println!("Final state: {:?}", result.final_state);
    println!("Steps taken: {}", result.steps);
    println!("Duration: {:?}", result.duration);
    println!("Success: {}", result.success);

    if let Some(reasoning_chain) = result
        .context
        .get_json::<serde_json::Value>("reasoning_chain")?
    {
        println!("\n🧠 Reasoning Chain:");
        println!("{}", serde_json::to_string_pretty(&reasoning_chain)?);
    }

    if let Some(execution_plan) = result
        .context
        .get_json::<serde_json::Value>("execution_plan")?
    {
        println!("\n📋 Execution Plan:");
        println!("{}", serde_json::to_string_pretty(&execution_plan)?);
    }

    let thoughts = result.context.get_recent_thoughts()?;
    if !thoughts.is_empty() {
        println!("\n🤔 Recent Thoughts:");
        for thought in thoughts {
            println!("  - {}", thought);
        }
    }

    Ok(())
}

// Mock MCP client for demonstration purposes
async fn create_mock_mcp_client(
) -> std::result::Result<Arc<dyn McpClient>, Box<dyn std::error::Error>> {
    // In a real implementation, this would create an actual MCP client
    // connecting to LLM services, planning services, etc.

    struct MockMcpClient;

    #[async_trait::async_trait]
    impl McpClient for MockMcpClient {
        async fn list_tools(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Tool>> {
            Ok(vec![])
        }

        async fn call_tool(
            &self,
            name: &str,
            _arguments: serde_json::Value,
        ) -> pocketflow_mcp::Result<serde_json::Value> {
            match name {
                "llm_reasoning" => Ok(serde_json::Value::String(
                    "Step 1: Analyze current system performance metrics\n\
                     Step 2: Identify bottlenecks in the system\n\
                     Step 3: Research optimization techniques\n\
                     Step 4: Evaluate impact of each technique\n\
                     Conclusion: Focus on database optimization and caching"
                        .to_string(),
                )),
                "planning_service" => Ok(serde_json::Value::String(
                    "1. Database query optimization\n\
                     2. Implement caching layer\n\
                     3. Code profiling and optimization\n\
                     4. Load balancing improvements\n\
                     5. Performance monitoring setup"
                        .to_string(),
                )),
                "llm_reflection" => Ok(serde_json::Value::String(
                    "The reasoning process was thorough and systematic. \
                     The plan addresses the main performance bottlenecks effectively."
                        .to_string(),
                )),
                _ => Ok(serde_json::Value::String("Mock response".to_string())),
            }
        }

        async fn list_resources(&self) -> pocketflow_mcp::Result<Vec<pocketflow_mcp::Resource>> {
            Ok(vec![])
        }

        async fn read_resource(&self, _uri: &str) -> pocketflow_mcp::Result<serde_json::Value> {
            Ok(serde_json::Value::Null)
        }

        async fn get_server_info(&self) -> pocketflow_mcp::Result<pocketflow_mcp::ServerInfo> {
            Ok(pocketflow_mcp::ServerInfo {
                name: "Mock Server".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                authors: None,
                homepage: None,
                license: None,
                repository: None,
            })
        }
    }

    Ok(Arc::new(MockMcpClient))
}
