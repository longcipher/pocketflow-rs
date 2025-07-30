use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use serde_json::Value;

use crate::error::{Result, ToolError};

/// Utility functions for tool development and management
pub mod tool_utils {
    use super::*;
    use crate::core::{ToolCapability, ToolParameters};

    /// Validate tool parameters against schema
    pub fn validate_parameters(params: &Value, schema: &ToolParameters) -> Result<()> {
        // Check required parameters
        // ToolParameters is a wrapper around serde_json::Value, so we need to extract the map if present
        if let Ok(obj) = schema.get_object("") {
            // OpenAPI-style: required is an array at the root, types are in each property
            let required_params: Vec<String> = obj
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if let Some(params_schema) = obj.get("properties").and_then(|v| v.as_object()) {
                // Check required parameters
                for param_name in &required_params {
                    if params.get(param_name).is_none() {
                        return Err(ToolError::invalid_parameters(format!(
                            "Missing required parameter: {param_name}"
                        )));
                    }
                }
                // Validate parameter types
                for (param_name, param_value) in
                    params.as_object().unwrap_or(&serde_json::Map::new())
                {
                    if let Some(param_info) = params_schema.get(param_name) {
                        let expected_type = param_info
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if !is_value_type_compatible(param_value, expected_type) {
                            return Err(ToolError::invalid_parameters(format!(
                                "Parameter '{}' has incorrect type. Expected: {}, Got: {}",
                                param_name,
                                expected_type,
                                get_value_type_name(param_value)
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a value is compatible with expected type
    fn is_value_type_compatible(value: &Value, expected_type: &str) -> bool {
        match expected_type {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => true, // Unknown types are allowed
        }
    }

    /// Get human-readable type name for a JSON value
    fn get_value_type_name(value: &Value) -> &'static str {
        match value {
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Null => "null",
        }
    }

    /// Extract nested value from JSON using dot notation
    pub fn get_nested_value<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;

        for part in parts {
            if let Some(index_start) = part.find('[') {
                // Handle array indexing like "items[0]"
                let field = &part[..index_start];
                let index_part = &part[index_start + 1..part.len() - 1];

                current = current.get(field)?;

                if let Ok(index) = index_part.parse::<usize>() {
                    current = current.get(index)?;
                } else {
                    return None;
                }
            } else {
                current = current.get(part)?;
            }
        }

        Some(current)
    }

    /// Set nested value in JSON using dot notation
    pub fn set_nested_value(data: &mut Value, path: &str, value: Value) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err(ToolError::invalid_parameters("Empty path"));
        }

        let mut current = data;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - set the value
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(part.to_string(), value);
                    return Ok(());
                } else {
                    return Err(ToolError::invalid_parameters(
                        "Cannot set value on non-object".to_string(),
                    ));
                }
            } else {
                // Intermediate part - navigate or create
                if current.get(part).is_none()
                    && let Some(obj) = current.as_object_mut()
                {
                    obj.insert(part.to_string(), Value::Object(serde_json::Map::new()));
                }
                current = current.get_mut(part).ok_or_else(|| {
                    ToolError::invalid_parameters(format!("Invalid path: {path}"))
                })?;
            }
        }

        Err(ToolError::invalid_parameters(
            "Failed to set nested value".to_string(),
        ))
    }

    /// Merge two JSON objects recursively
    pub fn merge_json_objects(base: &mut Value, overlay: &Value) {
        if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
            for (key, value) in overlay_obj {
                if let Some(base_value) = base_obj.get_mut(key) {
                    if base_value.is_object() && value.is_object() {
                        merge_json_objects(base_value, value);
                    } else {
                        *base_value = value.clone();
                    }
                } else {
                    base_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }

    /// Convert capabilities to human-readable strings
    pub fn capabilities_to_strings(capabilities: &[ToolCapability]) -> Vec<String> {
        capabilities
            .iter()
            .map(|cap| match cap {
                ToolCapability::Basic => "Basic".to_string(),
                ToolCapability::Streaming => "Streaming".to_string(),
                ToolCapability::Batch => "Batch".to_string(),
                ToolCapability::Authenticated => "Authenticated".to_string(),
                ToolCapability::Cacheable => "Cacheable".to_string(),
                ToolCapability::Idempotent => "Idempotent".to_string(),
                ToolCapability::LongRunning => "LongRunning".to_string(),
                ToolCapability::NetworkRequired => "NetworkRequired".to_string(),
                ToolCapability::StateMutating => "StateMutating".to_string(),
                ToolCapability::ReadOnly => "ReadOnly".to_string(),
            })
            .collect()
    }
}

/// Template processing utilities
pub mod template_utils {
    use regex::Regex;

    use super::*;

    /// Process template variables in text
    pub fn process_template(template: &str, variables: &HashMap<String, Value>) -> Result<String> {
        let var_regex =
            Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s*\}\}")
                .map_err(|e| ToolError::execution(format!("Regex error: {e}")))?;

        let mut result = template.to_string();

        for captures in var_regex.captures_iter(template) {
            if let Some(var_match) = captures.get(1) {
                let var_path = var_match.as_str();
                let replacement = resolve_template_variable(var_path, variables)?;
                let placeholder = &captures[0];
                result = result.replace(placeholder, &replacement);
            }
        }

        Ok(result)
    }

    /// Resolve a template variable from the variables map
    fn resolve_template_variable(path: &str, variables: &HashMap<String, Value>) -> Result<String> {
        let parts: Vec<&str> = path.split('.').collect();
        let root_var = parts[0];

        if let Some(value) = variables.get(root_var) {
            if parts.len() == 1 {
                // Simple variable
                Ok(value_to_string(value))
            } else {
                // Nested variable
                let nested_path = parts[1..].join(".");
                if let Some(nested_value) = tool_utils::get_nested_value(value, &nested_path) {
                    Ok(value_to_string(nested_value))
                } else {
                    Err(ToolError::invalid_parameters(format!(
                        "Variable path not found: {path}"
                    )))
                }
            }
        } else {
            Err(ToolError::invalid_parameters(format!(
                "Variable not found: {root_var}"
            )))
        }
    }

    /// Convert a JSON value to string for template replacement
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(_) | Value::Object(_) => value.to_string(),
        }
    }

    /// Check if a string contains template variables
    pub fn has_template_variables(text: &str) -> bool {
        text.contains("{{") && text.contains("}}")
    }

    /// Extract all template variable names from text
    pub fn extract_template_variables(text: &str) -> Result<Vec<String>> {
        let var_regex =
            Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s*\}\}")
                .map_err(|e| ToolError::execution(format!("Regex error: {e}")))?;

        let variables: Vec<String> = var_regex
            .captures_iter(text)
            .filter_map(|captures| captures.get(1).map(|m| m.as_str().to_string()))
            .collect();

        Ok(variables)
    }
}

/// Performance monitoring utilities
pub mod perf_utils {
    use super::*;

    /// Simple performance timer
    pub struct Timer {
        start: Instant,
        name: String,
    }

    impl Timer {
        pub fn new(name: &str) -> Self {
            Self {
                start: Instant::now(),
                name: name.to_string(),
            }
        }

        pub fn elapsed(&self) -> Duration {
            self.start.elapsed()
        }

        pub fn elapsed_ms(&self) -> u128 {
            self.elapsed().as_millis()
        }

        pub fn log_elapsed(&self) {
            tracing::info!("{} took {}ms", self.name, self.elapsed_ms());
        }
    }

    impl Drop for Timer {
        fn drop(&mut self) {
            self.log_elapsed();
        }
    }

    /// Performance metrics collector
    #[derive(Debug, Clone)]
    pub struct PerformanceMetrics {
        pub execution_time: Duration,
        pub memory_used: Option<u64>,
        pub operations_count: u64,
        pub success_rate: f64,
    }

    impl PerformanceMetrics {
        pub fn new() -> Self {
            Self {
                execution_time: Duration::from_millis(0),
                memory_used: None,
                operations_count: 0,
                success_rate: 0.0,
            }
        }

        pub fn with_execution_time(mut self, duration: Duration) -> Self {
            self.execution_time = duration;
            self
        }

        pub fn with_operations_count(mut self, count: u64) -> Self {
            self.operations_count = count;
            self
        }

        pub fn with_success_rate(mut self, rate: f64) -> Self {
            self.success_rate = rate;
            self
        }

        pub fn to_json(&self) -> Value {
            serde_json::json!({
                "execution_time_ms": self.execution_time.as_millis(),
                "memory_used_bytes": self.memory_used,
                "operations_count": self.operations_count,
                "success_rate": self.success_rate
            })
        }
    }

    impl Default for PerformanceMetrics {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Data conversion utilities
pub mod conversion_utils {
    use super::*;

    /// Convert between common data formats
    pub fn convert_data_format(data: &str, from_format: &str, to_format: &str) -> Result<String> {
        // Example: only support JSON to CSV for now
        if from_format.eq_ignore_ascii_case("json") && to_format.eq_ignore_ascii_case("csv") {
            return json_to_csv(data);
        }
        Err(ToolError::invalid_parameters(format!(
            "Unsupported conversion: {from_format} to {to_format}"
        )))
    }

    fn json_to_csv(json_data: &str) -> Result<String> {
        let json_value: Value = serde_json::from_str(json_data)
            .map_err(|e| ToolError::invalid_parameters(format!("Invalid JSON: {e}")))?;

        match json_value {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(String::new());
                }
                // Extract headers from first object
                let first_obj = arr[0].as_object().ok_or_else(|| {
                    ToolError::invalid_parameters("JSON array must contain objects")
                })?;
                let headers: Vec<String> = first_obj.keys().cloned().collect();
                let mut csv = headers.join(",") + "\n";
                // Add data rows
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let row: Vec<String> = headers
                            .iter()
                            .map(|h| {
                                obj.get(h).map_or("".to_string(), |v| match v {
                                    Value::String(s) => s.clone(),
                                    _ => v.to_string(),
                                })
                            })
                            .collect();
                        csv += &(row.join(",") + "\n");
                    }
                }
                Ok(csv)
            }
            _ => Err(ToolError::invalid_parameters(
                "JSON must be an array of objects for CSV conversion",
            )),
        }
    }

    pub fn validate_data_format(data: &str, format: &str) -> Result<()> {
        match format.to_lowercase().as_str() {
            "json" => {
                serde_json::from_str::<Value>(data)
                    .map_err(|e| ToolError::invalid_parameters(format!("Invalid JSON: {e}")))?;
            }
            "yaml" => {
                serde_yaml::from_str::<Value>(data)
                    .map_err(|e| ToolError::invalid_parameters(format!("Invalid YAML: {e}")))?;
            }
            "csv" => {
                // Basic CSV validation
                if data.lines().count() < 2 {
                    return Err(ToolError::invalid_parameters(
                        "CSV must have at least header and one data row",
                    ));
                }
            }
            _ => {
                return Err(ToolError::invalid_parameters(format!(
                    "Unknown format: {format}"
                )));
            }
        }
        Ok(())
    }
}

/// Error handling utilities
pub mod error_utils {
    use super::*;

    /// Retry function with exponential backoff
    pub async fn retry_with_backoff<F, Fut, T>(
        mut _operation: F,
        _max_attempts: usize,
        _initial_delay: Duration,
        _max_delay: Duration,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let _attempts = 0;
        let _delay = _initial_delay;

        // TODO: implement retry logic here
        // For now, just return a timeout error
        Err(ToolError::timeout("Retry attempts exhausted".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_nested_value_access() {
        let data = json!({
            "user": {
                "name": "John",
                "details": {
                    "age": 30,
                    "city": "New York"
                },
                "tags": ["developer", "rust"]
            }
        });

        assert_eq!(
            tool_utils::get_nested_value(&data, "user.name"),
            Some(&Value::String("John".to_string()))
        );

        assert_eq!(
            tool_utils::get_nested_value(&data, "user.details.age"),
            Some(&Value::Number(serde_json::Number::from(30)))
        );

        assert_eq!(tool_utils::get_nested_value(&data, "nonexistent"), None);
    }

    #[test]
    fn test_template_processing() {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), json!("World"));
        variables.insert("count".to_string(), json!(42));

        let template = "Hello {{ name }}! Count: {{ count }}";
        let result = template_utils::process_template(template, &variables).unwrap();

        assert_eq!(result, "Hello World! Count: 42");
    }

    #[test]
    fn test_performance_timer() {
        let timer = perf_utils::Timer::new("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.elapsed_ms() >= 10);
    }

    #[test]
    fn test_data_format_conversion() {
        let json_data = r#"[{"name": "John", "age": 30}]"#;
        let csv_result = conversion_utils::convert_data_format(json_data, "json", "csv").unwrap();

        println!("CSV result: {csv_result}");

        assert!(csv_result.contains("name"));
        assert!(csv_result.contains("age"));
        assert!(csv_result.contains("John"));
        assert!(csv_result.contains("30"));
    }
}
