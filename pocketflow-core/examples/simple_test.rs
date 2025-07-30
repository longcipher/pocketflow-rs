//! Simple test for PocketFlow-RS functionality.

use pocketflow_core::prelude::*;

// Simple workflow state
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum SimpleState {
    Start,
    End,
}

impl FlowState for SimpleState {
    fn is_terminal(&self) -> bool {
        matches!(self, SimpleState::End)
    }
}

// Simple node that transitions from Start to End
#[derive(Debug)]
struct SimpleNode;

#[async_trait]
impl Node for SimpleNode {
    type State = SimpleState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("Executing simple node...");
        context.set("executed", true)?;
        Ok((context, SimpleState::End))
    }

    fn name(&self) -> String {
        "simple_node".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting PocketFlow-RS simple test...");

    // Create a simple flow
    let flow = SimpleFlow::builder()
        .name("simple_test")
        .initial_state(SimpleState::Start)
        .node(SimpleState::Start, SimpleNode)
        .build()?;

    // Create context
    let context = Context::new();

    // Execute flow
    match flow.execute(context).await {
        Ok(result) => {
            println!("Flow executed successfully!");
            println!("Final state: {:?}", result.final_state);
            println!("Steps: {}", result.steps);
            println!("Duration: {:?}", result.duration);
            println!("Success: {}", result.success);
        }
        Err(e) => {
            println!("Flow execution failed: {}", e);
        }
    }

    Ok(())
}
