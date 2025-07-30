use pocketflow_core::prelude::Context;
use serde_json::Value;

use crate::error::{Result, ToolError};

/// Validation rules for tool parameters
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Field must exist
    FieldExists(String),
    /// Field must be of specific type
    FieldType(String, ValueType),
    /// Numeric field must be within range
    Range(String, f64, f64),
    /// String field must match pattern
    Pattern(String, String),
    /// Field must be one of allowed values
    Enum(String, Vec<Value>),
    /// Array field length constraints
    ArrayLength(String, Option<usize>, Option<usize>),
    /// String field length constraints
    StringLength(String, Option<usize>, Option<usize>),
    /// Custom validation function
    Custom(String, fn(&Value) -> bool),
}

impl ValidationRule {
    /// Create field existence rule
    pub fn field_exists(field: &str) -> Self {
        ValidationRule::FieldExists(field.to_string())
    }

    /// Create field type rule
    pub fn field_type(field: &str, value_type: ValueType) -> Self {
        ValidationRule::FieldType(field.to_string(), value_type)
    }

    /// Create range validation rule
    pub fn range(field: &str, min: f64, max: f64) -> Self {
        ValidationRule::Range(field.to_string(), min, max)
    }

    /// Create pattern validation rule
    pub fn pattern(field: &str, pattern: &str) -> Self {
        ValidationRule::Pattern(field.to_string(), pattern.to_string())
    }

    /// Create enum validation rule
    pub fn enum_values(field: &str, values: Vec<Value>) -> Self {
        ValidationRule::Enum(field.to_string(), values)
    }

    /// Create array length validation rule
    pub fn array_length(field: &str, min: Option<usize>, max: Option<usize>) -> Self {
        ValidationRule::ArrayLength(field.to_string(), min, max)
    }

    /// Create string length validation rule
    pub fn string_length(field: &str, min: Option<usize>, max: Option<usize>) -> Self {
        ValidationRule::StringLength(field.to_string(), min, max)
    }

    /// Create custom validation rule
    pub fn custom(field: &str, validator: fn(&Value) -> bool) -> Self {
        ValidationRule::Custom(field.to_string(), validator)
    }

    /// Validate a value against this rule
    pub fn validate(&self, data: &Value) -> Result<()> {
        match self {
            ValidationRule::FieldExists(field) => {
                if data.get(field).is_none() {
                    return Err(ToolError::invalid_parameters(format!(
                        "Required field '{field}' is missing"
                    )));
                }
                Ok(())
            }
            ValidationRule::FieldType(field, expected_type) => {
                if let Some(value) = data.get(field) {
                    let actual_type = ValueType::from_json_value(value);
                    if actual_type != *expected_type {
                        return Err(ToolError::invalid_parameters(format!(
                            "Field '{field}' expected type {expected_type:?}, got {actual_type:?}"
                        )));
                    }
                }
                Ok(())
            }
            ValidationRule::Range(field, min, max) => {
                if let Some(value) = data.get(field) {
                    if let Some(num) = value.as_f64() {
                        if num < *min || num > *max {
                            return Err(ToolError::invalid_parameters(format!(
                                "Field '{field}' value {num} is outside range [{min}, {max}]"
                            )));
                        }
                    } else {
                        return Err(ToolError::invalid_parameters(format!(
                            "Field '{field}' is not a number for range validation"
                        )));
                    }
                }
                Ok(())
            }
            ValidationRule::Pattern(field, pattern) => {
                if let Some(value) = data.get(field) {
                    if let Some(string_val) = value.as_str() {
                        let regex = regex::Regex::new(pattern).map_err(|e| {
                            ToolError::invalid_parameters(format!(
                                "Invalid regex pattern '{pattern}': {e}"
                            ))
                        })?;

                        if !regex.is_match(string_val) {
                            return Err(ToolError::invalid_parameters(format!(
                                "Field '{field}' value '{string_val}' does not match pattern '{pattern}'"
                            )));
                        }
                    } else {
                        return Err(ToolError::invalid_parameters(format!(
                            "Field '{field}' is not a string for pattern validation"
                        )));
                    }
                }
                Ok(())
            }
            ValidationRule::Enum(field, allowed_values) => {
                if let Some(value) = data.get(field)
                    && !allowed_values.contains(value)
                {
                    return Err(ToolError::invalid_parameters(format!(
                        "Field '{field}' value {value:?} is not in allowed values {allowed_values:?}"
                    )));
                }
                Ok(())
            }
            ValidationRule::ArrayLength(field, min, max) => {
                if let Some(value) = data.get(field) {
                    if let Some(array) = value.as_array() {
                        let len = array.len();

                        if let Some(min_len) = min
                            && len < *min_len
                        {
                            return Err(ToolError::invalid_parameters(format!(
                                "Field '{field}' array length {len} is less than minimum {min_len}"
                            )));
                        }

                        if let Some(max_len) = max
                            && len > *max_len
                        {
                            return Err(ToolError::invalid_parameters(format!(
                                "Field '{field}' array length {len} exceeds maximum {max_len}"
                            )));
                        }
                    } else {
                        return Err(ToolError::invalid_parameters(format!(
                            "Field '{field}' is not an array for length validation"
                        )));
                    }
                }
                Ok(())
            }
            ValidationRule::StringLength(field, min, max) => {
                if let Some(value) = data.get(field) {
                    if let Some(string_val) = value.as_str() {
                        let len = string_val.len();

                        if let Some(min_len) = min
                            && len < *min_len
                        {
                            return Err(ToolError::invalid_parameters(format!(
                                "Field '{field}' string length {len} is less than minimum {min_len}"
                            )));
                        }

                        if let Some(max_len) = max
                            && len > *max_len
                        {
                            return Err(ToolError::invalid_parameters(format!(
                                "Field '{field}' string length {len} exceeds maximum {max_len}"
                            )));
                        }
                    } else {
                        return Err(ToolError::invalid_parameters(format!(
                            "Field '{field}' is not a string for length validation"
                        )));
                    }
                }
                Ok(())
            }
            ValidationRule::Custom(field, validator) => {
                if let Some(value) = data.get(field)
                    && !validator(value)
                {
                    return Err(ToolError::invalid_parameters(format!(
                        "Field '{field}' failed custom validation"
                    )));
                }
                Ok(())
            }
        }
    }
}

/// Value types for validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Null,
}

impl ValueType {
    /// Determine value type from JSON value
    pub fn from_json_value(value: &Value) -> Self {
        match value {
            Value::String(_) => ValueType::String,
            Value::Number(_) => ValueType::Number,
            Value::Bool(_) => ValueType::Boolean,
            Value::Array(_) => ValueType::Array,
            Value::Object(_) => ValueType::Object,
            Value::Null => ValueType::Null,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ValueType::String => "string",
            ValueType::Number => "number",
            ValueType::Boolean => "boolean",
            ValueType::Array => "array",
            ValueType::Object => "object",
            ValueType::Null => "null",
        }
    }
}

/// Parameter validator for tool inputs
#[derive(Debug, Clone)]
pub struct ParameterValidator {
    rules: Vec<ValidationRule>,
}

impl ParameterValidator {
    /// Create new parameter validator
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add required field validation
    pub fn required_field(mut self, field: &str, value_type: ValueType) -> Self {
        self.rules.push(ValidationRule::field_exists(field));
        self.rules
            .push(ValidationRule::field_type(field, value_type));
        self
    }

    /// Add optional field validation
    pub fn optional_field(mut self, field: &str, value_type: ValueType) -> Self {
        self.rules
            .push(ValidationRule::field_type(field, value_type));
        self
    }

    /// Add range validation
    pub fn range_check(mut self, field: &str, min: f64, max: f64) -> Self {
        self.rules.push(ValidationRule::range(field, min, max));
        self
    }

    /// Add pattern validation
    pub fn pattern_check(mut self, field: &str, pattern: &str) -> Self {
        self.rules.push(ValidationRule::pattern(field, pattern));
        self
    }

    /// Add enum validation
    pub fn enum_check(mut self, field: &str, values: Vec<Value>) -> Self {
        self.rules.push(ValidationRule::enum_values(field, values));
        self
    }

    /// Add array length validation
    pub fn array_length_check(
        mut self,
        field: &str,
        min: Option<usize>,
        max: Option<usize>,
    ) -> Self {
        self.rules
            .push(ValidationRule::array_length(field, min, max));
        self
    }

    /// Add string length validation
    pub fn string_length_check(
        mut self,
        field: &str,
        min: Option<usize>,
        max: Option<usize>,
    ) -> Self {
        self.rules
            .push(ValidationRule::string_length(field, min, max));
        self
    }

    /// Add custom validation
    pub fn custom_check(mut self, field: &str, validator: fn(&Value) -> bool) -> Self {
        self.rules.push(ValidationRule::custom(field, validator));
        self
    }

    /// Validate data against all rules
    pub fn validate(&self, data: &Value) -> Result<()> {
        for rule in &self.rules {
            rule.validate(data)?;
        }
        Ok(())
    }

    /// Validate and collect all errors
    pub fn validate_all(&self, data: &Value) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        for rule in &self.rules {
            if let Err(e) = rule.validate(data) {
                errors.push(e.to_string());
            }
        }

        if errors.is_empty() {
            Ok(errors)
        } else {
            Err(ToolError::invalid_parameters(errors.join("; ")))
        }
    }
}

impl Default for ParameterValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Schema-based validator using JSON Schema
#[derive(Debug, Clone)]
pub struct SchemaValidator {
    schema: Value,
}

impl SchemaValidator {
    /// Create new schema validator
    pub fn new(schema: Value) -> Self {
        Self { schema }
    }

    /// Create from JSON Schema string
    pub fn from_json_str(schema_str: &str) -> Result<Self> {
        let schema: Value = serde_json::from_str(schema_str)
            .map_err(|e| ToolError::invalid_parameters(format!("Invalid JSON schema: {e}")))?;
        Ok(Self::new(schema))
    }

    /// Validate data against schema
    pub fn validate(&self, data: &Value) -> Result<()> {
        // This would use a proper JSON Schema validator like jsonschema crate
        // For now, basic implementation
        if let Some(required) = self.schema.get("required")
            && let Some(required_fields) = required.as_array()
        {
            for field in required_fields {
                if let Some(field_name) = field.as_str()
                    && data.get(field_name).is_none()
                {
                    return Err(ToolError::invalid_parameters(format!(
                        "Required field '{field_name}' is missing"
                    )));
                }
            }
        }

        if let Some(properties) = self.schema.get("properties")
            && let Some(props) = properties.as_object()
        {
            for (field_name, field_schema) in props {
                if let Some(field_value) = data.get(field_name) {
                    self.validate_field_against_schema(field_name, field_value, field_schema)?;
                }
            }
        }

        Ok(())
    }

    fn validate_field_against_schema(
        &self,
        field_name: &str,
        value: &Value,
        schema: &Value,
    ) -> Result<()> {
        if let Some(field_type) = schema.get("type")
            && let Some(expected_type) = field_type.as_str()
        {
            let actual_type = match value {
                Value::String(_) => "string",
                Value::Number(_) => "number",
                Value::Bool(_) => "boolean",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                Value::Null => "null",
            };

            if actual_type != expected_type {
                return Err(ToolError::invalid_parameters(format!(
                    "Field '{field_name}' expected type '{expected_type}', got '{actual_type}'"
                )));
            }
        }

        // Additional schema validations would go here
        Ok(())
    }
}

/// Validation node for workflows
use pocketflow_core::state::SimpleState;

pub struct ValidationNode {
    name: String,
    rules: Vec<ValidationRule>,
    on_success: SimpleState,
    on_error: SimpleState,
}

impl ValidationNode {
    /// Create validation node builder
    pub fn builder(name: &str) -> ValidationNodeBuilder {
        ValidationNodeBuilder::new(name)
    }

    /// Execute validation
    pub async fn execute(&self, context: &mut Context) -> Result<SimpleState> {
        // Get data to validate from context
        let data = serde_json::Value::Object(context.json_data().clone().into_iter().collect());

        // Run validation rules
        for rule in &self.rules {
            if let Err(e) = rule.validate(&data) {
                tracing::error!("Validation failed for {}: {}", self.name, e);
                return Ok(self.on_error.clone());
            }
        }

        tracing::info!("Validation passed for {}", self.name);
        Ok(self.on_success.clone())
    }
}

/// Builder for validation nodes
pub struct ValidationNodeBuilder {
    name: String,
    rules: Vec<ValidationRule>,
    on_success: Option<SimpleState>,
    on_error: Option<SimpleState>,
}

impl ValidationNodeBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            rules: Vec::new(),
            on_success: None,
            on_error: None,
        }
    }

    /// Add validation rules
    pub fn with_rules(mut self, rules: Vec<ValidationRule>) -> Self {
        self.rules = rules;
        self
    }

    /// Add single validation rule
    pub fn with_rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Set success state
    pub fn on_success(mut self, state: SimpleState) -> Self {
        self.on_success = Some(state);
        self
    }

    /// Set error state  
    pub fn on_error(mut self, state: SimpleState) -> Self {
        self.on_error = Some(state);
        self
    }

    /// Build validation node
    pub fn build(self) -> Result<ValidationNode> {
        let on_success = self
            .on_success
            .ok_or_else(|| ToolError::invalid_parameters("Missing success state"))?;
        let on_error = self
            .on_error
            .ok_or_else(|| ToolError::invalid_parameters("Missing error state"))?;

        Ok(ValidationNode {
            name: self.name,
            rules: self.rules,
            on_success,
            on_error,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_field_exists_validation() {
        let rule = ValidationRule::field_exists("name");
        let valid_data = json!({"name": "test"});
        let invalid_data = json!({"other": "value"});

        assert!(rule.validate(&valid_data).is_ok());
        assert!(rule.validate(&invalid_data).is_err());
    }

    #[test]
    fn test_field_type_validation() {
        let rule = ValidationRule::field_type("age", ValueType::Number);
        let valid_data = json!({"age": 25});
        let invalid_data = json!({"age": "twenty"});

        assert!(rule.validate(&valid_data).is_ok());
        assert!(rule.validate(&invalid_data).is_err());
    }

    #[test]
    fn test_range_validation() {
        let rule = ValidationRule::range("score", 0.0, 100.0);
        let valid_data = json!({"score": 85.5});
        let invalid_data = json!({"score": 150.0});

        assert!(rule.validate(&valid_data).is_ok());
        assert!(rule.validate(&invalid_data).is_err());
    }

    #[test]
    fn test_pattern_validation() {
        let rule = ValidationRule::pattern("email", r"^[^@]+@[^@]+\.[^@]+$");
        let valid_data = json!({"email": "test@example.com"});
        let invalid_data = json!({"email": "invalid-email"});

        assert!(rule.validate(&valid_data).is_ok());
        assert!(rule.validate(&invalid_data).is_err());
    }

    #[test]
    fn test_parameter_validator() {
        let validator = ParameterValidator::new()
            .required_field("name", ValueType::String)
            .optional_field("age", ValueType::Number)
            .range_check("age", 0.0, 150.0);

        let valid_data = json!({"name": "John", "age": 30});
        let invalid_data = json!({"age": 30}); // missing required name

        assert!(validator.validate(&valid_data).is_ok());
        assert!(validator.validate(&invalid_data).is_err());
    }
}
