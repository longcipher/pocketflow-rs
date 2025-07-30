use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use pocketflow_core::prelude::{Context, FlowError, FlowState, Node};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info};

use crate::{
    agent_node::AgentNode,
    agent_types::{AgentStep, StepType},
    error::{AgentError, Result},
};

/// Streaming execution states
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StreamingState {
    /// Ready to start streaming
    Ready,
    /// Currently streaming
    Streaming { tokens_received: usize },
    /// Stream paused
    Paused,
    /// Stream completed successfully
    Completed { total_tokens: usize },
    /// Stream failed
    Failed { error: String },
    /// Stream cancelled
    Cancelled,
}

impl FlowState for StreamingState {
    fn is_terminal(&self) -> bool {
        matches!(
            self,
            StreamingState::Completed { .. }
                | StreamingState::Failed { .. }
                | StreamingState::Cancelled
        )
    }
}

/// Stream chunk types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    /// Token/text chunk
    Token {
        content: String,
        position: usize,
        is_final: bool,
    },
    /// Agent step update
    Step { step: AgentStep, step_index: usize },
    /// Tool call started
    ToolCall {
        tool_name: String,
        arguments: serde_json::Value,
        call_id: String,
    },
    /// Tool call result
    ToolResult {
        call_id: String,
        result: serde_json::Value,
        success: bool,
    },
    /// Agent delegation
    Delegation {
        target_agent: String,
        task: String,
        delegation_id: String,
    },
    /// Thinking/reasoning step
    Thinking {
        content: String,
        reasoning_type: String,
    },
    /// Metadata update
    Metadata {
        key: String,
        value: serde_json::Value,
    },
    /// Error occurred
    Error { error: String, recoverable: bool },
    /// Stream ended
    End {
        final_result: Option<String>,
        success: bool,
    },
}

/// Streaming agent node that provides real-time updates
#[derive(Debug)]
pub struct StreamingAgentNode {
    agent: Arc<AgentNode>,
    name: String,
    buffer_size: usize,
    enable_step_streaming: bool,
    enable_thinking_streaming: bool,
    enable_tool_streaming: bool,
}

impl StreamingAgentNode {
    pub fn new(agent: Arc<AgentNode>) -> Self {
        let name = format!("streaming_{}", agent.config.name);
        Self {
            agent,
            name,
            buffer_size: 1000,
            enable_step_streaming: true,
            enable_thinking_streaming: true,
            enable_tool_streaming: true,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn enable_step_streaming(mut self, enable: bool) -> Self {
        self.enable_step_streaming = enable;
        self
    }

    pub fn enable_thinking_streaming(mut self, enable: bool) -> Self {
        self.enable_thinking_streaming = enable;
        self
    }

    pub fn enable_tool_streaming(mut self, enable: bool) -> Self {
        self.enable_tool_streaming = enable;
        self
    }

    /// Execute task with streaming updates
    pub async fn execute_streaming(
        &self,
        task: &str,
    ) -> Result<(impl Stream<Item = StreamChunk>, StreamingHandle)> {
        let (tx, rx) = mpsc::channel(self.buffer_size);
        let (control_tx, control_rx) = broadcast::channel(10);

        let agent = self.agent.clone();
        let task = task.to_string();
        let enable_step = self.enable_step_streaming;
        let enable_thinking = self.enable_thinking_streaming;
        let enable_tool = self.enable_tool_streaming;

        // Create streaming handle for control
        let handle = StreamingHandle {
            control_sender: control_tx.clone(),
            is_running: Arc::new(RwLock::new(false)),
        };

        let _control_sender = control_tx.clone();
        let is_running = handle.is_running.clone();

        // Spawn execution task
        tokio::spawn(async move {
            *is_running.write().await = true;

            if let Err(e) = Self::execute_with_streaming(
                agent,
                &task,
                tx,
                control_rx,
                enable_step,
                enable_thinking,
                enable_tool,
            )
            .await
            {
                error!("Streaming execution error: {}", e);
            }

            *is_running.write().await = false;
        });

        let stream = ReceiverStream::new(rx);
        Ok((stream, handle))
    }

    async fn execute_with_streaming(
        agent: Arc<AgentNode>,
        task: &str,
        sender: mpsc::Sender<StreamChunk>,
        mut control_rx: broadcast::Receiver<StreamControl>,
        enable_step: bool,
        enable_thinking: bool,
        enable_tool: bool,
    ) -> Result<()> {
        let mut _step_index = 0;
        let mut token_position = 0;
        let mut is_paused = false;

        info!("Starting streaming execution for task: {}", task);

        // Check for control signals
        macro_rules! check_control {
            () => {
                if let Ok(control) = control_rx.try_recv() {
                    match control {
                        StreamControl::Pause => {
                            is_paused = true;
                            debug!("Stream paused");
                        }
                        StreamControl::Resume => {
                            is_paused = false;
                            debug!("Stream resumed");
                        }
                        StreamControl::Cancel => {
                            let _ = sender
                                .send(StreamChunk::End {
                                    final_result: None,
                                    success: false,
                                })
                                .await;
                            return Ok(());
                        }
                    }
                }

                // Wait if paused
                while is_paused {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    if let Ok(control) = control_rx.try_recv() {
                        if matches!(control, StreamControl::Resume | StreamControl::Cancel) {
                            break;
                        }
                    }
                }
            };
        }

        // Start thinking stream if enabled
        if enable_thinking {
            let _ = sender
                .send(StreamChunk::Thinking {
                    content: format!("Starting to process task: {task}"),
                    reasoning_type: "initial_analysis".to_string(),
                })
                .await;
            check_control!();
        }

        // Execute the agent task with streaming
        match agent.execute_task(task).await {
            Ok(result) => {
                // Stream each step
                for (i, step) in result.steps.iter().enumerate() {
                    check_control!();

                    if enable_step {
                        let _ = sender
                            .send(StreamChunk::Step {
                                step: step.clone(),
                                step_index: i,
                            })
                            .await;
                    }

                    // Simulate streaming for different step types
                    match step.step_type {
                        StepType::ToolCall { tool_name: _ } => {
                            if enable_tool {
                                let _ =
                                    sender
                                        .send(StreamChunk::ToolCall {
                                            tool_name: "tool".to_string(),
                                            arguments: serde_json::Value::Object(
                                                serde_json::Map::new(),
                                            ),
                                            call_id: format!("call_{i}"),
                                        })
                                        .await;

                                // Simulate tool execution time
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                check_control!();

                                let _ = sender
                                    .send(StreamChunk::ToolResult {
                                        call_id: format!("call_{i}"),
                                        result: step
                                            .output
                                            .clone()
                                            .unwrap_or(serde_json::Value::Null),
                                        success: true,
                                    })
                                    .await;
                            }
                        }
                        StepType::Delegation { target_agent: _ } => {
                            let _ = sender
                                .send(StreamChunk::Delegation {
                                    target_agent: "unknown_agent".to_string(),
                                    task: step.input.as_str().unwrap_or("").to_string(),
                                    delegation_id: format!("delegation_{i}"),
                                })
                                .await;
                        }
                        StepType::Thinking => {
                            if enable_thinking {
                                let content = step
                                    .output
                                    .as_ref()
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Thinking...");
                                let _ = sender
                                    .send(StreamChunk::Thinking {
                                        content: content.to_string(),
                                        reasoning_type: "step_reasoning".to_string(),
                                    })
                                    .await;
                            }
                        }
                        StepType::Response => {
                            // Stream response tokens
                            if let Some(output) = &step.output
                                && let Some(content) = output.as_str()
                            {
                                let words: Vec<&str> = content.split_whitespace().collect();
                                for (word_idx, word) in words.iter().enumerate() {
                                    check_control!();

                                    let _ = sender
                                        .send(StreamChunk::Token {
                                            content: if word_idx == 0 {
                                                word.to_string()
                                            } else {
                                                format!(" {word}")
                                            },
                                            position: token_position,
                                            is_final: word_idx == words.len() - 1,
                                        })
                                        .await;

                                    token_position += 1;

                                    // Simulate typing delay
                                    tokio::time::sleep(tokio::time::Duration::from_millis(50))
                                        .await;
                                }
                            }
                        }
                        _ => {
                            // For other types, just stream the content from output
                            if let Some(output) = &step.output
                                && let Some(content_str) = output.as_str()
                            {
                                let _ = sender
                                    .send(StreamChunk::Token {
                                        content: content_str.to_string(),
                                        position: token_position,
                                        is_final: true,
                                    })
                                    .await;
                                token_position += 1;
                            }
                        }
                    }

                    _step_index += 1;
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }

                // Send final result
                let _ = sender
                    .send(StreamChunk::End {
                        final_result: result.final_answer.clone(),
                        success: result.success,
                    })
                    .await;

                info!("Streaming execution completed successfully");
            }
            Err(e) => {
                error!("Agent execution failed: {}", e);
                let _ = sender
                    .send(StreamChunk::Error {
                        error: e.to_string(),
                        recoverable: false,
                    })
                    .await;

                let _ = sender
                    .send(StreamChunk::End {
                        final_result: None,
                        success: false,
                    })
                    .await;
            }
        }

        Ok(())
    }

    /// Execute with custom stream processor
    pub async fn execute_with_processor<F>(
        &self,
        task: &str,
        processor: F,
    ) -> Result<StreamingResult>
    where
        F: Fn(StreamChunk) -> bool + Send + Sync + 'static,
    {
        let (stream, _handle) = self.execute_streaming(task).await?;
        let mut chunks = Vec::new();
        let mut final_result = None;
        let mut success = false;
        let mut total_tokens = 0;

        let mut stream = Box::pin(stream);
        while let Some(chunk) = stream.next().await {
            let should_continue = processor(chunk.clone());

            match &chunk {
                StreamChunk::Token { position, .. } => {
                    total_tokens = position + 1;
                }
                StreamChunk::End {
                    final_result: result,
                    success: s,
                } => {
                    final_result = result.clone();
                    success = *s;
                }
                _ => {}
            }

            chunks.push(chunk);

            if !should_continue {
                break;
            }
        }

        Ok(StreamingResult {
            chunks,
            final_result,
            success,
            total_tokens,
        })
    }
}

#[async_trait]
impl Node for StreamingAgentNode {
    type State = StreamingState;

    async fn execute(
        &self,
        context: Context,
    ) -> std::result::Result<(Context, Self::State), FlowError> {
        // Extract task from context
        let task: String = context
            .get_json("task")?
            .and_then(|v: serde_json::Value| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| FlowError::context("No task provided in context"))?;

        // Execute streaming
        let result = self.execute_streaming(&task).await;
        match result {
            Ok((stream, _handle)) => {
                let mut new_context = context;
                let mut chunks = Vec::new();
                let mut total_tokens = 0;
                let mut final_result = None;
                let mut success = false;

                // Collect all chunks
                let mut stream = Box::pin(stream);
                while let Some(chunk) = stream.next().await {
                    match &chunk {
                        StreamChunk::Token { position, .. } => {
                            total_tokens = *position + 1;
                        }
                        StreamChunk::End {
                            final_result: result,
                            success: s,
                        } => {
                            final_result = result.clone();
                            success = *s;
                        }
                        _ => {}
                    }
                    chunks.push(chunk);
                }

                // Store results in context
                new_context.set("stream_chunks", serde_json::to_value(&chunks)?)?;
                new_context.set("total_tokens", total_tokens)?;

                if success {
                    if let Some(result) = final_result {
                        new_context.set("final_answer", &result)?;
                    }
                    Ok((new_context, StreamingState::Completed { total_tokens }))
                } else {
                    Ok((
                        new_context,
                        StreamingState::Failed {
                            error: "Streaming execution failed".to_string(),
                        },
                    ))
                }
            }
            Err(e) => {
                let mut new_context = context;
                new_context.set("error", e.to_string())?;
                Ok((
                    new_context,
                    StreamingState::Failed {
                        error: e.to_string(),
                    },
                ))
            }
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Handle for controlling streaming execution
#[derive(Debug, Clone)]
pub struct StreamingHandle {
    control_sender: broadcast::Sender<StreamControl>,
    is_running: Arc<RwLock<bool>>,
}

impl StreamingHandle {
    pub async fn pause(&self) -> Result<()> {
        self.control_sender
            .send(StreamControl::Pause)
            .map_err(|_| AgentError::streaming("Failed to send pause signal"))?;
        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        self.control_sender
            .send(StreamControl::Resume)
            .map_err(|_| AgentError::streaming("Failed to send resume signal"))?;
        Ok(())
    }

    pub async fn cancel(&self) -> Result<()> {
        self.control_sender
            .send(StreamControl::Cancel)
            .map_err(|_| AgentError::streaming("Failed to send cancel signal"))?;
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

/// Control signals for streaming
#[derive(Debug, Clone)]
enum StreamControl {
    Pause,
    Resume,
    Cancel,
}

/// Result of streaming execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResult {
    pub chunks: Vec<StreamChunk>,
    pub final_result: Option<String>,
    pub success: bool,
    pub total_tokens: usize,
}

impl StreamingResult {
    pub fn get_text_content(&self) -> String {
        self.chunks
            .iter()
            .filter_map(|chunk| {
                if let StreamChunk::Token { content, .. } = chunk {
                    Some(content.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_steps(&self) -> Vec<&AgentStep> {
        self.chunks
            .iter()
            .filter_map(|chunk| {
                if let StreamChunk::Step { step, .. } = chunk {
                    Some(step)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_tool_calls(&self) -> Vec<(String, serde_json::Value)> {
        self.chunks
            .iter()
            .filter_map(|chunk| {
                if let StreamChunk::ToolCall {
                    tool_name,
                    arguments,
                    ..
                } = chunk
                {
                    Some((tool_name.clone(), arguments.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn had_errors(&self) -> bool {
        self.chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::Error { .. }))
    }
}

/// Builder for StreamingAgentNode
pub struct StreamingAgentNodeBuilder {
    agent: Option<Arc<AgentNode>>,
    name: Option<String>,
    buffer_size: usize,
    enable_step_streaming: bool,
    enable_thinking_streaming: bool,
    enable_tool_streaming: bool,
}

impl StreamingAgentNodeBuilder {
    pub fn new() -> Self {
        Self {
            agent: None,
            name: None,
            buffer_size: 1000,
            enable_step_streaming: true,
            enable_thinking_streaming: true,
            enable_tool_streaming: true,
        }
    }

    pub fn with_agent(mut self, agent: Arc<AgentNode>) -> Self {
        self.agent = Some(agent);
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn enable_step_streaming(mut self, enable: bool) -> Self {
        self.enable_step_streaming = enable;
        self
    }

    pub fn enable_thinking_streaming(mut self, enable: bool) -> Self {
        self.enable_thinking_streaming = enable;
        self
    }

    pub fn enable_tool_streaming(mut self, enable: bool) -> Self {
        self.enable_tool_streaming = enable;
        self
    }

    pub fn build(self) -> Result<StreamingAgentNode> {
        let agent = self
            .agent
            .ok_or_else(|| AgentError::configuration("Agent is required for streaming node"))?;

        let mut streaming_node = StreamingAgentNode::new(agent)
            .with_buffer_size(self.buffer_size)
            .enable_step_streaming(self.enable_step_streaming)
            .enable_thinking_streaming(self.enable_thinking_streaming)
            .enable_tool_streaming(self.enable_tool_streaming);

        if let Some(name) = self.name {
            streaming_node = streaming_node.with_name(name);
        }

        Ok(streaming_node)
    }
}

impl Default for StreamingAgentNodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::AgentNodeBuilder;

    #[tokio::test]
    async fn test_streaming_node_creation() {
        let agent = Arc::new(
            AgentNodeBuilder::new("test_agent", "Test agent")
                .with_openai_model("gpt-4o-mini")
                .build()
                .await
                .unwrap(),
        );

        let streaming_node = StreamingAgentNodeBuilder::new()
            .with_agent(agent)
            .with_name("test_streaming")
            .with_buffer_size(500)
            .build()
            .unwrap();

        assert_eq!(streaming_node.name(), "test_streaming");
        assert_eq!(streaming_node.buffer_size, 500);
    }

    #[tokio::test]
    async fn test_streaming_handle() {
        let (tx, _rx) = broadcast::channel(10);
        let handle = StreamingHandle {
            control_sender: tx,
            is_running: Arc::new(RwLock::new(true)),
        };

        assert!(handle.is_running().await);
        assert!(handle.pause().await.is_ok());
        assert!(handle.resume().await.is_ok());
        assert!(handle.cancel().await.is_ok());
    }

    #[test]
    fn test_streaming_result_methods() {
        let chunks = vec![
            StreamChunk::Token {
                content: "Hello".to_string(),
                position: 0,
                is_final: false,
            },
            StreamChunk::Token {
                content: " world".to_string(),
                position: 1,
                is_final: true,
            },
            StreamChunk::ToolCall {
                tool_name: "search".to_string(),
                arguments: serde_json::json!({"query": "test"}),
                call_id: "call_1".to_string(),
            },
        ];

        let result = StreamingResult {
            chunks,
            final_result: Some("Hello world".to_string()),
            success: true,
            total_tokens: 2,
        };

        assert_eq!(result.get_text_content(), "Hello world");
        assert_eq!(result.get_tool_calls().len(), 1);
        assert!(!result.had_errors());
    }
}
