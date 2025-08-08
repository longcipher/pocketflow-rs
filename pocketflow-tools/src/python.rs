use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::timeout,
};

use crate::{
    core::{Tool, ToolCapability, ToolCategory, ToolContext, ToolParameters, ToolResult},
    error::{Result, ToolError},
};

/// Execute Python code in a subprocess with timeout and capture.
///
/// Parameters:
/// - code (string, required): Python code to run. If both `code` and `file` provided, `code` wins.
/// - file (string, optional): Path to a Python file to execute.
/// - args (array[string], optional): Arguments passed to Python.
/// - python_path (string, optional): Explicit python executable path; defaults to "python3".
/// - timeout_ms (number, optional): Kill the process if it exceeds this timeout (default 15000).
/// - workdir (string, optional): Working directory for the process.
/// - env (object, optional): Extra environment variables.
/// - simulate (boolean, optional): If true, do not execute; return a stub result.
pub struct PythonExecutionTool;

impl PythonExecutionTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonExecutionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PythonExecutionTool {
    fn name(&self) -> &str {
        "python_execute"
    }
    fn description(&self) -> &str {
        "Execute Python code or scripts in a sandboxed subprocess"
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    fn parameter_schema(&self) -> Value {
        ToolParameters::new_schema()
            .add_optional("code", "string", "Inline Python code to execute", None)
            .add_optional("file", "string", "Path to a Python file to execute", None)
            .add_optional("args", "array", "Arguments for the Python process", None)
            .add_optional(
                "python_path",
                "string",
                "Python executable (default python3)",
                Some(json!("python3")),
            )
            .add_optional(
                "timeout_ms",
                "number",
                "Process timeout in milliseconds",
                Some(json!(15000)),
            )
            .add_optional("workdir", "string", "Working directory", None)
            .add_optional("env", "object", "Extra environment variables", None)
            .add_optional("stdin", "string", "String to pass via stdin", None)
            .add_optional(
                "stdin_json",
                "object",
                "JSON to pass via stdin (serialized)",
                None,
            )
            .add_optional(
                "simulate",
                "boolean",
                "Return a stubbed result without executing",
                Some(json!(false)),
            )
            .into()
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::StateMutating, ToolCapability::LongRunning]
    }

    async fn execute(&self, params: ToolParameters, _ctx: ToolContext) -> Result<ToolResult> {
        let simulate: Option<bool> = params.get_optional("simulate")?;
        if simulate.unwrap_or(false) {
            let content = json!({
                "simulated": true,
                "stdout": "",
                "stderr": "",
                "status": 0
            })
            .to_string();
            return Ok(
                ToolResult::success(content).with_content_type(crate::core::ContentType::Json)
            );
        }

        let code: Option<String> = params.get_optional("code")?;
        let file: Option<String> = params.get_optional("file")?;
        if code.is_none() && file.is_none() {
            return Err(ToolError::invalid_parameters(
                "Either 'code' or 'file' must be provided",
            ));
        }

        let python_path: String = params
            .get_optional("python_path")?
            .unwrap_or_else(|| "python3".to_string());
        let args_list: Vec<String> = params
            .get_optional::<Vec<String>>("args")?
            .unwrap_or_default();
        let workdir: Option<String> = params.get_optional("workdir")?;
        let timeout_ms: u64 = params.get_optional("timeout_ms")?.unwrap_or(15000);
        let env_map_opt: Option<serde_json::Map<String, Value>> = params.get_optional("env")?;
        let stdin_str: Option<String> = params.get_optional("stdin")?;
        let stdin_json: Option<serde_json::Value> = params.get_optional("stdin_json")?;

        // Pre-compute stdin bytes if provided (string or JSON)
        let stdin_bytes: Option<Vec<u8>> = match (&stdin_json, &stdin_str) {
            (Some(v), _) => serde_json::to_vec(v).ok(),
            (None, Some(s)) => Some(s.clone().into_bytes()),
            _ => None,
        };

        // Build command
        let mut cmd = Command::new(python_path);
        if let Some(dir) = &workdir {
            cmd.current_dir(dir);
        }
        if let Some(env_map) = env_map_opt {
            for (k, v) in env_map.into_iter() {
                if let Some(s) = v.as_str() {
                    cmd.env(k, s);
                }
            }
        }

        if let Some(code_str) = code {
            cmd.arg("-c").arg(code_str);
        } else if let Some(file_path) = file {
            cmd.arg(file_path);
        }
        for a in args_list {
            cmd.arg(a);
        }

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError::execution(format!("Failed to spawn python: {e}")))?;

        // Write to stdin if provided (collapse conditions)
        if let (Some(mut sin), Some(bytes)) = (child.stdin.take(), stdin_bytes) {
            if !bytes.is_empty() {
                let _ = sin.write_all(&bytes).await;
            }
            let _ = sin.shutdown().await;
        }

        let fut = async {
            let status = child
                .wait()
                .await
                .map_err(|e| ToolError::execution(e.to_string()))?;
            let mut stdout = String::new();
            if let Some(mut out) = child.stdout.take() {
                let mut buf = Vec::new();
                let _ = out.read_to_end(&mut buf).await;
                stdout = String::from_utf8_lossy(&buf).to_string();
            }
            let mut stderr = String::new();
            if let Some(mut err) = child.stderr.take() {
                let mut buf = Vec::new();
                let _ = err.read_to_end(&mut buf).await;
                stderr = String::from_utf8_lossy(&buf).to_string();
            }
            Ok::<(i32, String, String), ToolError>((status.code().unwrap_or(-1), stdout, stderr))
        };

        let out = timeout(std::time::Duration::from_millis(timeout_ms), fut)
            .await
            .map_err(|_| ToolError::timeout("python execution timed out"))?;

        let (code_rc, stdout, stderr) = out?;
        // Try to parse stdout as JSON
        let mut content_obj = json!({
            "status": code_rc,
            "stdout": stdout,
            "stderr": stderr
        });
        let parsed = content_obj["stdout"]
            .as_str()
            .and_then(|s| serde_json::from_str::<Value>(s).ok());
        if let Some(val) = parsed {
            // Attach parsed JSON for consumers
            content_obj["stdout_json"] = val;
        }
        let content = content_obj.to_string();

        let mut result =
            ToolResult::success(content).with_content_type(crate::core::ContentType::Json);
        if code_rc != 0 {
            result = result.with_metadata("exit_code", json!(code_rc));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ToolContext;

    #[tokio::test]
    async fn test_python_simulated() {
        let tool = PythonExecutionTool::new();
        let params = ToolParameters::new(json!({"simulate": true}));
        let ctx = ToolContext::new();
        let res = tool.execute(params, ctx).await.unwrap();
        assert!(res.is_success());
        assert_eq!(res.content_type, crate::core::ContentType::Json);
    }

    #[tokio::test]
    async fn test_python_inline() {
        let tool = PythonExecutionTool::new();
        let params = ToolParameters::new(json!({"code": "print(1+1)", "timeout_ms": 5000}));
        let ctx = ToolContext::new();
        let res = tool.execute(params, ctx).await.unwrap();
        assert!(res.is_success());
        assert!(res.content.contains("\"stdout\""));
    }

    #[tokio::test]
    async fn test_python_stdin_json_and_parse() {
        let tool = PythonExecutionTool::new();
        let code = r#"import sys, json
data = json.load(sys.stdin)
print(json.dumps({"sum": data.get("a",0) + data.get("b",0)}))"#;
        let params = ToolParameters::new(json!({
            "code": code,
            "stdin_json": {"a": 2, "b": 3},
            "timeout_ms": 5000
        }));
        let ctx = ToolContext::new();
        let res = tool.execute(params, ctx).await.unwrap();
        assert!(res.is_success());
        let v: Value = serde_json::from_str(&res.content).unwrap();
        assert_eq!(v["stdout_json"]["sum"], json!(5));
    }
}
