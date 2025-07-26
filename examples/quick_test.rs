//! Quick test to verify our framework works.

use pocketflow_rs::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TestState {
    Start,
    End,
}

impl FlowState for TestState {
    fn is_terminal(&self) -> bool {
        matches!(self, TestState::End)
    }
}

#[derive(Debug)]
struct SimpleNode;

#[async_trait]
impl Node for SimpleNode {
    type State = TestState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        context.set("result", "success")?;
        Ok((context, TestState::End))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing PocketFlow-RS");

    let flow = pocketflow_rs::flow::SimpleFlow::builder()
        .name("TestFlow")
        .initial_state(TestState::Start)
        .node(TestState::Start, SimpleNode)
        .build()?;

    let context = Context::new();
    let result = flow.execute(context).await?;

    println!("✅ Test passed! Final state: {:?}", result.final_state);
    println!("Success: {}", result.success);

    Ok(())
}
