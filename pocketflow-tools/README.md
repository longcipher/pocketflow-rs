# PocketFlow Tools

[![Crates.io](https://img.shields.io/crates/v/pocketflow-tools.svg)](https://crates.io/crates/pocketflow-tools)
[![Documentation](https://docs.rs/pocketflow-tools/badge.svg)](https://docs.rs/pocketflow-tools)
[![License](https://img.shields.io/crates/l/pocketflow-tools.svg)](https://github.com/longcipher/pocketflow-rs/blob/master/LICENSE)

Tool system for PocketFlow workflow automation with JSON schema validation and execution framework.

## Features

- **Tool Registry**: Centralized tool management and execution
- **JSON Schema Validation**: Automatic parameter validation using JSON schemas
- **Async Execution**: Full async/await support for non-blocking operations
- **HTTP Tools**: Built-in HTTP client tools for web API integration
- **Custom Tools**: Easy framework for creating domain-specific tools
- **Error Handling**: Comprehensive error reporting with detailed context
- **Parameter Builder**: Type-safe parameter construction with validation

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
pocketflow-tools = "0.2.0"
pocketflow-core = "0.2.0"
```

## Basic Usage

### Creating a Custom Tool

```rust
use pocketflow_tools::{Tool, ToolContext, ToolResult, ToolParameters};
use async_trait::async_trait;
use serde_json::json;

pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }
    
    fn description(&self) -> &str {
        "Performs basic arithmetic operations"
    }
    
    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First operand"
                },
                "b": {
                    "type": "number",
                    "description": "Second operand"
                }
            },
            "required": ["operation", "a", "b"]
        })
    }
    
    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let operation: String = params.get("operation")?;
        let a: f64 = params.get("a")?;
        let b: f64 = params.get("b")?;
        
        let result = match operation.as_str() {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return ToolResult::error("Division by zero");
                }
                a / b
            }
            _ => return ToolResult::error("Invalid operation"),
        };
        
        ToolResult::success(json!({ "result": result }))
    }
}
```

### Using Tool Registry

```rust
use pocketflow_tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ToolRegistry::new();
    
    // Register the tool
    registry.register_tool(Box::new(CalculatorTool))?;
    
    // Execute a tool
    let params = json!({
        "operation": "add",
        "a": 5.0,
        "b": 3.0
    });
    
    let result = registry.execute_tool("calculator", params).await?;
    println!("Result: {:?}", result);
    
    Ok(())
}
```

## Parameter Builder

Use the parameter builder for type-safe parameter construction:

```rust
use pocketflow_tools::ToolParameters;

fn create_parameters() -> Result<ToolParameters, Box<dyn std::error::Error>> {
    let params = ToolParameters::builder()
        .add_required("input", "Hello, World!")
        .add_optional("format", "uppercase")
        .add_number("count", 5)
        .add_boolean("verbose", true)
        .build()?;
    
    // Access parameters with type safety
    let input: String = params.get("input")?;
    let format: Option<String> = params.get_optional("format")?;
    let count: i32 = params.get("count")?;
    let verbose: bool = params.get("verbose")?;
    
    Ok(params)
}
```

## HTTP Tools

Built-in HTTP tools for web API integration:

```rust
use pocketflow_tools::{HttpTool, ToolRegistry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ToolRegistry::new();
    
    // Register HTTP tools
    registry.register_tool(Box::new(HttpTool::new()))?;
    
    // Make an HTTP request
    let params = json!({
        "method": "GET",
        "url": "https://api.github.com/users/octocat",
        "headers": {
            "User-Agent": "pocketflow-tools/0.2.0"
        }
    });
    
    let response = registry.execute_tool("http_request", params).await?;
    println!("Response: {:?}", response);
    
    Ok(())
}
```

## Tool Context

Tools receive execution context for advanced scenarios:

```rust
use pocketflow_tools::{Tool, ToolContext, ToolResult, ToolParameters};

pub struct FileProcessor;

#[async_trait]
impl Tool for FileProcessor {
    // ... other methods ...
    
    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Access execution environment
        let timeout = context.timeout();
        let retry_count = context.retry_count();
        
        // Access custom metadata
        if let Some(workspace_root) = context.get_metadata("workspace_root") {
            println!("Working in: {}", workspace_root);
        }
        
        // Perform tool operation with context awareness
        // ...
        
        ToolResult::success(json!({"processed": true}))
    }
}
```

## Validation and Error Handling

Comprehensive parameter validation and error reporting:

```rust
use pocketflow_tools::{ToolError, ToolResult};

// Parameter validation is automatic based on JSON schema
let result = match params.get::<String>("required_field") {
    Ok(value) => {
        // Process the validated value
        ToolResult::success(json!({"value": value}))
    }
    Err(e) => {
        // Handle validation error
        ToolResult::error(&format!("Validation failed: {}", e))
    }
};

// Custom validation
if let Some(email) = params.get_optional::<String>("email")? {
    if !email.contains('@') {
        return ToolResult::error("Invalid email format");
    }
}
```

## Integration with Workflows

Tools integrate seamlessly with PocketFlow workflows:

```rust
use pocketflow_core::{Context, Node, FlowState};
use pocketflow_tools::ToolRegistry;
use async_trait::async_trait;

pub struct ToolExecutorNode {
    registry: ToolRegistry,
    tool_name: String,
}

#[async_trait]
impl Node for ToolExecutorNode {
    type State = MyState;
    
    async fn execute(&self, mut context: Context) -> Result<(Context, Self::State), FlowError> {
        // Get parameters from context
        let params = context.get_json("tool_params")?;
        
        // Execute tool
        match self.registry.execute_tool(&self.tool_name, params).await {
            Ok(result) => {
                context.set("tool_result", result)?;
                Ok((context, MyState::Success))
            }
            Err(e) => {
                context.set("error", e.to_string())?;
                Ok((context, MyState::Error))
            }
        }
    }
    
    fn name(&self) -> String {
        format!("tool_executor_{}", self.tool_name)
    }
}
```

## Schema Generation

Automatic JSON schema generation for tool parameters:

```rust
use pocketflow_tools::schema::ToolParametersSchema;

let schema = ToolParametersSchema::new()
    .add_required("name", "string", "The user's name")
    .add_optional("age", "integer", "The user's age", Some(json!(25)))
    .add_enum("role", vec!["admin", "user", "guest"], "User role")
    .add_array("skills", "string", "List of skills")
    .add_object("address", json!({
        "type": "object",
        "properties": {
            "street": {"type": "string"},
            "city": {"type": "string"}
        }
    }), "User address")
    .build();
```

## Cargo Features

- `http` (default): Enable HTTP client tools
- `full`: Enable all features

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
