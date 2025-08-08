//! Plan execution node that iterates an ExecutionPlan and executes steps via MCP tools.

use std::sync::Arc;

use async_trait::async_trait;
use pocketflow_core::{context::Context, node::Node, state::FlowState};
use pocketflow_mcp::client::McpClient;
use serde_json::{Value, json};
use tokio::time::{Duration, sleep};

use crate::{Result, traits::ExecutionPlan};

/// Executes each step in `execution_plan` by invoking a configured MCP tool.
/// Stores per-step results in `execution_results` and updates `completed_steps`.
pub struct PlanExecutionNode<S: FlowState> {
    name: String,
    mcp_client: Arc<dyn McpClient>,
    tool_name: String,
    stop_on_error: bool,
    max_retries: usize,
    initial_backoff_ms: u64,
    enforce_success_criteria: bool,
    success_state: S,
    error_state: S,
}

impl<S: FlowState> std::fmt::Debug for PlanExecutionNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanExecutionNode")
            .field("name", &self.name)
            .field("tool_name", &self.tool_name)
            .field("stop_on_error", &self.stop_on_error)
            .finish()
    }
}

impl<S: FlowState> PlanExecutionNode<S> {
    pub fn builder() -> PlanExecutionNodeBuilder<S> {
        PlanExecutionNodeBuilder::new()
    }
}

#[async_trait]
impl<S: FlowState> Node for PlanExecutionNode<S> {
    type State = S;

    async fn execute(
        &self,
        mut context: Context,
    ) -> pocketflow_core::error::Result<(Context, Self::State)> {
        let plan: ExecutionPlan = if let Some(p) = context.get_json("execution_plan")? {
            p
        } else {
            context.set("execution_error", "No execution_plan in context")?;
            return Ok((context, self.error_state.clone()));
        };

        // Results vector of objects { step_id, status, output }
        let mut results: Vec<serde_json::Value> =
            context.get_json("execution_results")?.unwrap_or_default();
        let mut completed: Vec<String> = context.get_json("completed_steps")?.unwrap_or_default();

        for step in &plan.steps {
            // Compute effective policies with per-step overrides
            let enforce_success = step
                .enforce_success_criteria
                .unwrap_or(self.enforce_success_criteria);
            let max_retries = step.max_retries.unwrap_or(self.max_retries);
            let initial_backoff_ms = step.initial_backoff_ms.unwrap_or(self.initial_backoff_ms);
            let stop_on_error = step.stop_on_error.unwrap_or(self.stop_on_error);
            let mut attempt = 0;
            loop {
                let ctx_json = context.to_json()?; // JSON-only snapshot of context
                let args = json!({
                    "step_id": step.id,
                    "description": step.description,
                    "required_tools": step.required_tools,
                    "context": ctx_json,
                });

                match self.mcp_client.call_tool(&self.tool_name, args).await {
                    Ok(output) => {
                        // Optionally check rich success criteria
                        if enforce_success {
                            let text = output.as_str().unwrap_or("");
                            // Best-effort parse the output as JSON for JSON-pointer checks
                            let parsed_json: Option<Value> = if let Some(s) = output.as_str() {
                                serde_json::from_str::<Value>(s).ok()
                            } else if output.is_object() || output.is_array() {
                                Some(output.clone())
                            } else {
                                None
                            };

                            let mut all_ok = true;
                            for crit in &step.success_criteria {
                                let ok = if let Some(s) = crit.as_str() {
                                    // substring match
                                    text.contains(s)
                                } else if let Some(regex) =
                                    crit.get("regex").and_then(|v| v.as_str())
                                {
                                    match regex::Regex::new(regex) {
                                        Ok(re) => re.is_match(text),
                                        Err(_) => false,
                                    }
                                } else if let Some(ptr) =
                                    crit.get("json_pointer").and_then(|v| v.as_str())
                                {
                                    if let Some(json) = &parsed_json {
                                        let target = json.pointer(ptr);
                                        if let Some(eq_val) = crit.get("equals") {
                                            target.map(|v| v == eq_val).unwrap_or(false)
                                        } else if crit
                                            .get("exists")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false)
                                        {
                                            target.is_some()
                                        } else if let Some(substr) =
                                            crit.get("contains").and_then(|v| v.as_str())
                                        {
                                            match target {
                                                Some(Value::String(s)) => s.contains(substr),
                                                Some(Value::Array(arr)) => arr.iter().any(|e| {
                                                    e.as_str()
                                                        .map(|s| s.contains(substr))
                                                        .unwrap_or(false)
                                                }),
                                                _ => false,
                                            }
                                        } else {
                                            target.is_some()
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };
                                if !ok {
                                    all_ok = false;
                                    break;
                                }
                            }

                            if !all_ok {
                                results.push(json!({ "step_id": step.id, "status": "error", "error": "success criteria not met", "output": output }));
                                context.set("execution_results", &results)?;
                                context.set("last_step_error", "success criteria not met")?;
                                if stop_on_error {
                                    return Ok((context, self.error_state.clone()));
                                }
                                break;
                            }
                        }
                        results
                            .push(json!({ "step_id": step.id, "status": "ok", "output": output }));
                        completed.push(step.id.clone());
                        context.set("execution_results", &results)?;
                        context.set("completed_steps", &completed)?;
                        break;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        attempt += 1;
                        if attempt > max_retries {
                            results.push(json!({ "step_id": step.id, "status": "error", "error": err_str.clone() }));
                            context.set("execution_results", &results)?;
                            context.set("last_step_error", err_str)?;
                            if stop_on_error {
                                return Ok((context, self.error_state.clone()));
                            }
                            break;
                        } else {
                            let backoff = initial_backoff_ms * (1u64 << (attempt - 1));
                            sleep(Duration::from_millis(backoff)).await;
                        }
                    }
                }
            }
        }

        context.set("execution_completed", true)?;
        Ok((context, self.success_state.clone()))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Builder for PlanExecutionNode
pub struct PlanExecutionNodeBuilder<S: FlowState> {
    name: Option<String>,
    mcp_client: Option<Arc<dyn McpClient>>,
    tool_name: String,
    stop_on_error: bool,
    max_retries: usize,
    initial_backoff_ms: u64,
    enforce_success_criteria: bool,
    success_state: Option<S>,
    error_state: Option<S>,
}

impl<S: FlowState> PlanExecutionNodeBuilder<S> {
    pub fn new() -> Self {
        Self {
            name: None,
            mcp_client: None,
            tool_name: "execute_step".to_string(),
            stop_on_error: true,
            max_retries: 2,
            initial_backoff_ms: 200,
            enforce_success_criteria: false,
            success_state: None,
            error_state: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn with_mcp_client(mut self, client: Arc<dyn McpClient>) -> Self {
        self.mcp_client = Some(client);
        self
    }
    pub fn tool_name(mut self, tool: impl Into<String>) -> Self {
        self.tool_name = tool.into();
        self
    }
    pub fn stop_on_error(mut self, b: bool) -> Self {
        self.stop_on_error = b;
        self
    }
    pub fn max_retries(mut self, n: usize) -> Self {
        self.max_retries = n;
        self
    }
    pub fn initial_backoff_ms(mut self, ms: u64) -> Self {
        self.initial_backoff_ms = ms;
        self
    }
    pub fn enforce_success_criteria(mut self, b: bool) -> Self {
        self.enforce_success_criteria = b;
        self
    }
    pub fn on_success(mut self, s: S) -> Self {
        self.success_state = Some(s);
        self
    }
    pub fn on_error(mut self, s: S) -> Self {
        self.error_state = Some(s);
        self
    }

    pub fn build(self) -> Result<PlanExecutionNode<S>> {
        let name = self.name.unwrap_or_else(|| "plan_executor".to_string());
        let client = self.mcp_client.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("MCP client is required")
        })?;
        let success_state = self.success_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Success state is required")
        })?;
        let error_state = self.error_state.ok_or_else(|| {
            pocketflow_core::error::FlowError::construction("Error state is required")
        })?;

        Ok(PlanExecutionNode {
            name,
            mcp_client: client,
            tool_name: self.tool_name,
            stop_on_error: self.stop_on_error,
            max_retries: self.max_retries,
            initial_backoff_ms: self.initial_backoff_ms,
            enforce_success_criteria: self.enforce_success_criteria,
            success_state,
            error_state,
        })
    }
}

impl<S: FlowState> Default for PlanExecutionNodeBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
