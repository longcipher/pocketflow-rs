//! Minimal test for PocketFlow-RS.

use pocketflow_rs::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TestState {
    Start,
    End,
}

impl FlowState for TestState {
    fn is_terminal(&self) -> bool {
        matches!(self, TestState::End)
    }
}

struct TestNode;

#[async_trait]
impl Node for TestNode {
    type State = TestState;

    async fn execute(&self, context: Context) -> Result<(Context, Self::State)> {
        println!("Test node executed!");
        Ok((context, TestState::End))
    }

    fn name(&self) -> String {
        "test".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing PocketFlow-RS...");

    let flow = SimpleFlow::builder()
        .name("test")
        .initial_state(TestState::Start)
        .node(TestState::Start, TestNode)
        .build()?;

    let context = Context::new();
    let result = flow.execute(context).await?;

    println!("Success! Final state: {:?}", result.final_state);
    println!("Steps: {}, Duration: {:?}", result.steps, result.duration);

    Ok(())
}
