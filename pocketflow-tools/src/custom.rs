use async_trait::async_trait;
use serde_json::json;

use crate::{
    core::{Tool, ToolCapability, ToolCategory, ToolContext, ToolParameters, ToolResult},
    error::{Result, ToolError},
};

/// Example of a custom tool that can be used as a template
pub struct CustomExampleTool {
    name: String,
    description: String,
}

impl CustomExampleTool {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[async_trait]
impl Tool for CustomExampleTool {
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
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Message to process"
                }
            },
            "required": ["message"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Basic]
    }

    async fn execute(
        &self,
        parameters: ToolParameters,
        _context: ToolContext,
    ) -> Result<ToolResult> {
        let message = parameters
            .get::<String>("message")
            .map_err(|_| ToolError::invalid_parameters("Missing 'message' parameter"))?;

        let result = format!("Processed: {message}");
        Ok(ToolResult::success(result))
    }
}

/// Simple text processor tool
pub struct TextProcessorTool {
    name: String,
    processor: Box<dyn Fn(&str) -> String + Send + Sync>,
}

impl TextProcessorTool {
    pub fn new<F>(name: impl Into<String>, processor: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            processor: Box::new(processor),
        }
    }
}

#[async_trait]
impl Tool for TextProcessorTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Text processing tool"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Custom
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to process"
                }
            },
            "required": ["text"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Basic]
    }

    async fn execute(
        &self,
        parameters: ToolParameters,
        _context: ToolContext,
    ) -> Result<ToolResult> {
        let text = parameters
            .get::<String>("text")
            .map_err(|_| ToolError::invalid_parameters("Missing 'text' parameter"))?;

        let result = (self.processor)(&text);
        Ok(ToolResult::success(result))
    }
}

/// Helper functions for creating common custom tools
pub mod helpers {
    use super::*;

    /// Create a text processor tool
    pub fn text_processor_tool<F>(name: &str, processor: F) -> TextProcessorTool
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        TextProcessorTool::new(name, processor)
    }

    /// Create an uppercase converter tool
    pub fn uppercase_tool() -> TextProcessorTool {
        text_processor_tool("uppercase", |text| text.to_uppercase())
    }

    /// Create a lowercase converter tool
    pub fn lowercase_tool() -> TextProcessorTool {
        text_processor_tool("lowercase", |text| text.to_lowercase())
    }

    /// Create a word count tool
    pub fn word_count_tool() -> TextProcessorTool {
        text_processor_tool("word_count", |text| {
            text.split_whitespace().count().to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ToolContext;

    #[tokio::test]
    async fn test_custom_example_tool() {
        let tool = CustomExampleTool::new("test_tool", "A test tool");
        let params = ToolParameters::new(json!({"message": "hello"}));
        let context = ToolContext::new();

        let result = tool.execute(params, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content, "Processed: hello");
    }

    #[tokio::test]
    async fn test_text_processor_tool() {
        let tool = helpers::uppercase_tool();
        let params = ToolParameters::new(json!({"text": "hello world"}));
        let context = ToolContext::new();

        let result = tool.execute(params, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content, "HELLO WORLD");
    }

    #[tokio::test]
    async fn test_word_count_tool() {
        let tool = helpers::word_count_tool();
        let params = ToolParameters::new(json!({"text": "hello world test"}));
        let context = ToolContext::new();

        let result = tool.execute(params, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content, "3");
    }
}
