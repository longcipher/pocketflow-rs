//! Example demonstrating MCP integration with PocketFlow-RS.
//!
//! This example shows how to:
//! - Create workflow nodes that call external MCP tools
//! - Set up MCP servers that expose workflow functionality
//! - Use the MCP registry to manage connections
//! - Chain MCP operations in a workflow

use std::{process::Command, sync::Arc};

use pocketflow_mcp::{
    client::{McpClientNode, McpTransportConfig},
    context::McpContextExt,
    prelude::*,
    registry::global,
    server::{BasicWorkflowToolProvider, WorkflowMcpHandler},
};

// Define workflow states
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum McpWorkflowState {
    Start,
    ReadingFile,
    ProcessingContent,
    AnalyzingData,
    StoringResults,
    Success,
    Error,
}

impl FlowState for McpWorkflowState {
    fn is_terminal(&self) -> bool {
        matches!(self, McpWorkflowState::Success | McpWorkflowState::Error)
    }

    fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (McpWorkflowState::Start, McpWorkflowState::ReadingFile) => true,
            (McpWorkflowState::ReadingFile, McpWorkflowState::ProcessingContent) => true,
            (McpWorkflowState::ReadingFile, McpWorkflowState::Error) => true,
            (McpWorkflowState::ProcessingContent, McpWorkflowState::AnalyzingData) => true,
            (McpWorkflowState::ProcessingContent, McpWorkflowState::Error) => true,
            (McpWorkflowState::AnalyzingData, McpWorkflowState::StoringResults) => true,
            (McpWorkflowState::AnalyzingData, McpWorkflowState::Error) => true,
            (McpWorkflowState::StoringResults, McpWorkflowState::Success) => true,
            (McpWorkflowState::StoringResults, McpWorkflowState::Error) => true,
            _ => false,
        }
    }
}

/// Custom node that sets up MCP connections
#[derive(Debug)]
struct McpSetupNode;

#[async_trait]
impl Node for McpSetupNode {
    type State = McpWorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🔧 Setting up MCP connections...");

        // In a real implementation, you would create actual MCP clients here
        // For this example, we'll just simulate the setup

        context.set("mcp_setup_complete", true)?;
        context.set("file_path", "/tmp/example.txt")?;

        println!("✅ MCP connections established");
        Ok((context, McpWorkflowState::ReadingFile))
    }

    fn name(&self) -> String {
        "MCP Setup".to_string()
    }
}

/// Node that processes file content using MCP
#[derive(Debug)]
struct ContentProcessingNode;

#[async_trait]
impl Node for ContentProcessingNode {
    type State = McpWorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("📝 Processing file content...");

        // Simulate reading file content (in reality, this would use MCP filesystem tools)
        let file_content = "This is example file content that needs to be analyzed.";
        context.set("file_content", file_content)?;

        // Simulate content processing
        let processed_content = format!("Processed: {}", file_content);
        context.set("processed_content", processed_content)?;

        println!("✅ Content processed successfully");
        Ok((context, McpWorkflowState::AnalyzingData))
    }

    fn name(&self) -> String {
        "Content Processing".to_string()
    }
}

/// Node that analyzes data using MCP AI tools
#[derive(Debug)]
struct DataAnalysisNode;

#[async_trait]
impl Node for DataAnalysisNode {
    type State = McpWorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("🔍 Analyzing data with AI tools...");

        let content: String = context.get_json("processed_content")?.unwrap_or_default();

        // Simulate AI analysis (would use MCP AI tools in reality)
        let analysis_result = serde_json::json!({
            "sentiment": "positive",
            "key_topics": ["example", "content", "analysis"],
            "confidence": 0.85,
            "summary": "This is a positive example demonstrating content analysis."
        });

        context.set("analysis_result", analysis_result)?;

        println!("✅ Data analysis completed");
        Ok((context, McpWorkflowState::StoringResults))
    }

    fn name(&self) -> String {
        "Data Analysis".to_string()
    }
}

/// Node that stores results using MCP database tools
#[derive(Debug)]
struct ResultStorageNode;

#[async_trait]
impl Node for ResultStorageNode {
    type State = McpWorkflowState;

    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State)> {
        println!("💾 Storing results...");

        let analysis: serde_json::Value = context.get_json("analysis_result")?.unwrap_or_default();

        // Simulate storing to database (would use MCP database tools in reality)
        let storage_result = serde_json::json!({
            "id": "analysis_001",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": "stored"
        });

        context.set("storage_result", storage_result)?;
        context.set("workflow_complete", true)?;

        println!("✅ Results stored successfully");
        Ok((context, McpWorkflowState::Success))
    }

    fn name(&self) -> String {
        "Result Storage".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting PocketFlow-RS MCP Integration Example");

    // Create the workflow
    let flow = SimpleFlow::builder()
        .name("MCP Workflow Example")
        .initial_state(McpWorkflowState::Start)
        .node(McpWorkflowState::Start, McpSetupNode)
        .node(McpWorkflowState::ReadingFile, ContentProcessingNode)
        .node(McpWorkflowState::AnalyzingData, DataAnalysisNode)
        .node(McpWorkflowState::StoringResults, ResultStorageNode)
        .build()?;

    // Create initial context
    let mut context = Context::new();
    context.set_metadata("workflow_id", "mcp_example_001")?;
    context.set_metadata("start_time", chrono::Utc::now().to_rfc3339())?;

    println!("\n📋 Initial context setup complete");

    // Execute the workflow
    println!("\n🏃 Executing MCP-integrated workflow...");
    let result = flow.execute(context).await?;

    // Display results
    println!("\n📊 Workflow Execution Results:");
    println!("Final State: {:?}", result.final_state);
    println!("Execution Duration: {:?}", result.duration);
    println!("Steps Executed: {}", result.steps);
    println!("Success: {}", result.success);

    if let Some(error) = &result.error {
        println!("Error: {}", error);
    }

    // Show final context data
    println!("\n📄 Final Context Data:");

    if let Some(setup_complete) = result.context.get_json::<bool>("mcp_setup_complete")? {
        println!("MCP Setup Complete: {}", setup_complete);
    }

    if let Some(content) = result.context.get_json::<String>("processed_content")? {
        println!("Processed Content: {}", content);
    }

    if let Some(analysis) = result
        .context
        .get_json::<serde_json::Value>("analysis_result")?
    {
        println!(
            "Analysis Result: {}",
            serde_json::to_string_pretty(&analysis)?
        );
    }

    if let Some(storage) = result
        .context
        .get_json::<serde_json::Value>("storage_result")?
    {
        println!(
            "Storage Result: {}",
            serde_json::to_string_pretty(&storage)?
        );
    }

    println!("\n✅ MCP integration example completed successfully!");

    // Demonstrate MCP registry usage
    demonstrate_mcp_registry().await?;

    Ok(())
}

/// Demonstrate how to use the MCP registry for managing connections
async fn demonstrate_mcp_registry() -> Result<()> {
    println!("\n🔧 Demonstrating MCP Registry Usage:");

    // Note: In a real implementation, you would create actual MCP clients
    // and register them with the global registry like this:

    // let filesystem_client = RmcpClient::new(McpTransportConfig::ChildProcess(
    //     Command::new("npx").arg("-y").arg("@modelcontextprotocol/server-filesystem")
    // )).await?;
    //
    // global::register_client("filesystem", Arc::new(filesystem_client)).await?;

    println!("📝 MCP clients would be registered here");
    println!("🔍 Tools would be discovered and cataloged");
    println!("⚡ Workflow nodes would use registered clients");

    // Demonstrate tool provider setup
    let tool_provider = Arc::new(BasicWorkflowToolProvider::new());
    let tools = tool_provider
        .get_tools()
        .await
        .map_err(|e| FlowError::context(e.to_string()))?;

    println!("\n🛠️ Available Workflow Tools:");
    for tool in tools {
        println!(
            "  - {}: {}",
            tool.name,
            tool.description.unwrap_or_default()
        );
    }

    println!("\n💡 MCP registry demonstration complete!");

    Ok(())
}
