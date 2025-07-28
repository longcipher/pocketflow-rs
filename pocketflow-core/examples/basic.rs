//! Basic usage example of PocketFlow-RS.

use pocketflow_core::prelude::*;

// Define your workflow state
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum WorkflowState {
    Start,
    Processing,
    Validating,
    Success,
    Error,
}

impl FlowState for WorkflowState {
    fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Success | WorkflowState::Error)
    }

    fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (WorkflowState::Start, WorkflowState::Processing) => true,
            (WorkflowState::Processing, WorkflowState::Validating) => true,
            (WorkflowState::Validating, WorkflowState::Success | WorkflowState::Error) => true,
            _ => false,
        }
    }
}

// Define a processing node
#[derive(Debug)]
struct ProcessNode {
    name: String,
}

impl ProcessNode {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for ProcessNode {
    type State = WorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🔄 Processing in node: {}", self.name);

        let input: String = context.get_json("input")?.unwrap_or_default();

        if input.is_empty() {
            context.set("error", "No input provided")?;
            return Ok((context, WorkflowState::Error));
        }

        let processed = format!("Processed by {}: {}", self.name, input);
        context.set("processed_data", processed)?;

        Ok((context, WorkflowState::Processing))
    }
}

// Define a validation node
#[derive(Debug)]
struct ValidationNode {
    name: String,
}

impl ValidationNode {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for ValidationNode {
    type State = WorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("✅ Validating in node: {}", self.name);

        let processed_data: String = context.get_json("processed_data")?.unwrap_or_default();

        if processed_data.is_empty() {
            context.set("error", "No processed data to validate")?;
            return Ok((context, WorkflowState::Error));
        }

        context.set("validation_result", "Data is valid")?;

        Ok((context, WorkflowState::Validating))
    }
}

// Define a finalization node
#[derive(Debug)]
struct FinalizeNode;

#[async_trait]
impl Node for FinalizeNode {
    type State = WorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🏁 Finalizing workflow");

        let validation: String = context.get_json("validation_result")?.unwrap_or_default();

        if validation.is_empty() {
            context.set("error", "No validation result")?;
            return Ok((context, WorkflowState::Error));
        }

        context.set("final_result", "Workflow completed successfully")?;

        Ok((context, WorkflowState::Success))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting PocketFlow-RS Basic Example");

    // Create a simple flow using the builder
    let flow = SimpleFlow::builder()
        .name("BasicWorkflow")
        .initial_state(WorkflowState::Start)
        // Add nodes for each state
        .node(WorkflowState::Start, ProcessNode::new("DataProcessor"))
        .node(
            WorkflowState::Processing,
            ValidationNode::new("DataValidator"),
        )
        .node(WorkflowState::Validating, FinalizeNode)
        .build()?;

    // Create initial context with input data
    let mut context = Context::new();
    context.set("input", "Hello, PocketFlow!")?;
    context.set_metadata("workflow_id", "example-001")?;

    println!("\n📋 Initial context:");
    println!("Input: {:?}", context.get_json::<String>("input")?);

    // Execute the flow
    println!("\n🏃 Executing workflow...");
    let result = flow.execute(context).await?;

    // Display results
    println!("\n📊 Workflow Results:");
    println!("Final State: {:?}", result.final_state);
    println!("Execution Duration: {:?}", result.duration);
    println!("Steps Executed: {}", result.steps);
    println!("Success: {}", result.success);

    if let Some(error) = &result.error {
        println!("Error: {}", error);
    }

    println!("\n📄 Final Context Data:");
    if let Some(processed) = result.context.get_json::<String>("processed_data")? {
        println!("Processed Data: {}", processed);
    }
    if let Some(validation) = result.context.get_json::<String>("validation_result")? {
        println!("Validation Result: {}", validation);
    }
    if let Some(final_result) = result.context.get_json::<String>("final_result")? {
        println!("Final Result: {}", final_result);
    }

    println!("\n✅ Basic example completed successfully!");

    Ok(())
}
