use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{
    core::{Tool, ToolCapability, ToolCategory, ToolContext, ToolParameters, ToolResult},
    error::{Result, ToolError},
};

/// A simple web search/fetch tool that performs an HTTP GET to a configurable endpoint.
///
/// Notes:
/// - This tool does not require API keys by default. You can pass a custom `endpoint` that
///   returns JSON and optional `headers` to integrate with a search API.
/// - For testing or offline runs, set `simulate=true` and it will return a stubbed response.
///
/// Parameters:
/// - query (string, required): The search query.
/// - endpoint (string, optional): Endpoint URL template. Use `{query}` placeholder for the query.
///   Defaults to DuckDuckGo HTML: "https://duckduckgo.com/?q={query}".
/// - headers (object, optional): Map of header key -> value strings.
/// - limit (number, optional): Max bytes to return from the body (default 8192).
/// - simulate (boolean, optional): If true, returns a stubbed result and performs no network.
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Perform a simple web search/fetch via HTTP GET"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }

    fn parameter_schema(&self) -> Value {
        ToolParameters::new_schema()
            .add_required("query", "string", "Search query")
            .add_optional(
                "endpoint",
                "string",
                "Endpoint URL template with {query}",
                None,
            )
            .add_optional("headers", "object", "Request headers map", None)
            .add_optional(
                "limit",
                "number",
                "Max bytes of response to include",
                Some(json!(8192)),
            )
            .add_optional(
                "simulate",
                "boolean",
                "Return a stubbed result without network",
                Some(json!(false)),
            )
            .into()
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::NetworkRequired, ToolCapability::ReadOnly]
    }

    async fn execute(&self, params: ToolParameters, _ctx: ToolContext) -> Result<ToolResult> {
        let query: String = params.get("query")?;
        let simulate: Option<bool> = params.get_optional("simulate")?;
        if simulate.unwrap_or(false) {
            let content = json!({
                "query": query,
                "results": [
                    {"title": "Example Result 1", "url": "https://example.com/1", "snippet": "This is a simulated result."},
                    {"title": "Example Result 2", "url": "https://example.com/2", "snippet": "Another simulated result."}
                ],
                "simulated": true
            })
            .to_string();
            return Ok(
                ToolResult::success(content).with_content_type(crate::core::ContentType::Json)
            );
        }

        // Network call path is compiled only when http feature is enabled
        #[cfg(feature = "http")]
        {
            let default_endpoint = "https://duckduckgo.com/?q={query}".to_string();
            let endpoint: Option<String> = params.get_optional("endpoint")?;
            let limit: Option<usize> = params.get_optional("limit")?;
            let headers_val: Option<serde_json::Map<String, Value>> =
                params.get_optional("headers")?;

            let url = endpoint
                .unwrap_or(default_endpoint)
                .replace("{query}", &urlencoding::encode(&query));

            let client = reqwest::Client::new();
            let mut req = client.get(&url);
            if let Some(hmap) = headers_val {
                for (k, v) in hmap.into_iter() {
                    if let Some(s) = v.as_str() {
                        req = req.header(k, s.to_string());
                    }
                }
            }

            let resp = req
                .send()
                .await
                .map_err(|e| ToolError::network(e.to_string()))?;
            let status = resp.status().as_u16();
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| ToolError::execution(e.to_string()))?;
            let max = limit.unwrap_or(8192);
            let slice = if bytes.len() > max {
                &bytes[..max]
            } else {
                &bytes
            };
            let body_snippet = String::from_utf8_lossy(slice).to_string();

            let content = json!({
                "query": query,
                "status": status,
                "body": body_snippet,
                "endpoint": url,
            })
            .to_string();
            return Ok(
                ToolResult::success(content).with_content_type(crate::core::ContentType::Json)
            );
        }

        #[cfg(not(feature = "http"))]
        {
            Err(ToolError::configuration(
                "HTTP feature not enabled for web_search tool",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ToolContext;

    #[tokio::test]
    async fn test_simulated_search() {
        let tool = WebSearchTool::new();
        let params = ToolParameters::new(json!({"query": "pocketflow", "simulate": true}));
        let ctx = ToolContext::new();
        let res = tool.execute(params, ctx).await.unwrap();
        assert!(res.is_success());
        assert_eq!(res.content_type, crate::core::ContentType::Json);
        assert!(res.content.contains("simulated"));
    }
}
