use anyhow::Error;
use database::querying::QueryParameterDefinition;
use rmcp::model::CallToolResult;
use rmcp::model::{JsonObject, Tool};
use serde_json::Value;

pub trait KnowledgeGraphTool: Send + Sync {
    fn name(&self) -> &str;
    fn to_mcp_tool(&self) -> Tool;
    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::Error>;
}

pub enum ToolParameterDefinition {
    String(Option<String>),
    Int(Option<i64>),
    Number(Option<f64>),
    Boolean(Option<bool>),
}

impl ToolParameterDefinition {
    pub fn from_query_kind(kind: QueryParameterDefinition) -> ToolParameterDefinition {
        match kind {
            QueryParameterDefinition::String(value) => ToolParameterDefinition::String(value),
            QueryParameterDefinition::Int(value) => ToolParameterDefinition::Int(value),
            QueryParameterDefinition::Float(value) => ToolParameterDefinition::Number(value),
            QueryParameterDefinition::Boolean(value) => ToolParameterDefinition::Boolean(value),
        }
    }

    pub fn to_mcp_tool_type(&self) -> String {
        match self {
            ToolParameterDefinition::String(_) => "string".to_string(),
            ToolParameterDefinition::Int(_) => "integer".to_string(),
            ToolParameterDefinition::Number(_) => "number".to_string(),
            ToolParameterDefinition::Boolean(_) => "boolean".to_string(),
        }
    }

    pub fn to_mcp_tool_default(&self) -> Option<Value> {
        match self {
            ToolParameterDefinition::String(value) => value.clone().map(Value::String),
            ToolParameterDefinition::Int(value) => (*value).map(Value::from),
            ToolParameterDefinition::Number(value) => (*value).map(Value::from),
            ToolParameterDefinition::Boolean(value) => (*value).map(Value::Bool),
        }
    }
}

pub struct ToolParameter {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub definition: ToolParameterDefinition,
}

impl ToolParameter {
    pub fn to_mcp_tool_parameter(&self) -> serde_json::Value {
        let mut fields = serde_json::Map::new();
        fields.insert(
            "description".to_string(),
            serde_json::Value::String(self.description.to_string()),
        );

        fields.insert(
            "type".to_string(),
            serde_json::Value::String(self.definition.to_mcp_tool_type()),
        );

        if let Some(default) = self.definition.to_mcp_tool_default() {
            fields.insert("default".to_string(), default);
        }

        serde_json::Value::Object(fields)
    }

    pub fn get_value(&self, params: JsonObject) -> Result<Value, Error> {
        let value = params.get(self.name);

        if value.is_none() {
            if self.required {
                return Err(anyhow::anyhow!(
                    "Parameter {} is required but not provided.",
                    self.name
                ));
            }

            if let Some(default) = self.definition.to_mcp_tool_default() {
                return Ok(default);
            }

            return Ok(serde_json::Value::Null);
        }

        Ok(value.unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod tool_parameter_definition_tests {
        use super::*;

        #[test]
        fn test_from_query_kind_conversion() {
            assert!(matches!(
                ToolParameterDefinition::from_query_kind(QueryParameterDefinition::String(None)),
                ToolParameterDefinition::String(None)
            ));
            assert!(matches!(
                ToolParameterDefinition::from_query_kind(QueryParameterDefinition::Int(None)),
                ToolParameterDefinition::Int(None)
            ));
            assert!(matches!(
                ToolParameterDefinition::from_query_kind(QueryParameterDefinition::Float(None)),
                ToolParameterDefinition::Number(None)
            ));
            assert!(matches!(
                ToolParameterDefinition::from_query_kind(QueryParameterDefinition::Boolean(None)),
                ToolParameterDefinition::Boolean(None)
            ));
        }

        #[test]
        fn test_to_mcp_tool_type_mapping() {
            assert_eq!(
                ToolParameterDefinition::String(None).to_mcp_tool_type(),
                "string"
            );
            assert_eq!(
                ToolParameterDefinition::Int(None).to_mcp_tool_type(),
                "integer"
            );
            assert_eq!(
                ToolParameterDefinition::Number(None).to_mcp_tool_type(),
                "number"
            );
            assert_eq!(
                ToolParameterDefinition::Boolean(None).to_mcp_tool_type(),
                "boolean"
            );
        }

        #[test]
        fn test_to_mcp_tool_default_value_mapping() {
            assert_eq!(
                ToolParameterDefinition::String(Some("test".to_string())).to_mcp_tool_default(),
                Some(Value::String("test".to_string()))
            );
            assert_eq!(
                ToolParameterDefinition::Int(Some(10)).to_mcp_tool_default(),
                Some(Value::from(10))
            );
            assert_eq!(
                ToolParameterDefinition::Number(Some(10.0)).to_mcp_tool_default(),
                Some(Value::from(10.0))
            );
            assert_eq!(
                ToolParameterDefinition::Boolean(Some(true)).to_mcp_tool_default(),
                Some(Value::Bool(true))
            );
        }

        #[test]
        fn test_to_mcp_tool_default_value_mapping_with_none() {
            assert_eq!(
                ToolParameterDefinition::String(None).to_mcp_tool_default(),
                None
            );
            assert_eq!(
                ToolParameterDefinition::Int(None).to_mcp_tool_default(),
                None
            );
            assert_eq!(
                ToolParameterDefinition::Number(None).to_mcp_tool_default(),
                None
            );
            assert_eq!(
                ToolParameterDefinition::Boolean(None).to_mcp_tool_default(),
                None
            );
        }
    }

    mod tool_parameter_tests {
        use super::*;

        fn create_test_parameter(
            required: bool,
            definition: ToolParameterDefinition,
        ) -> ToolParameter {
            ToolParameter {
                name: "test_param",
                description: "A test parameter",
                required,
                definition,
            }
        }

        #[test]
        fn test_to_mcp_tool_parameter_without_default() {
            let param = create_test_parameter(true, ToolParameterDefinition::String(None));

            let result = param.to_mcp_tool_parameter();

            assert_eq!(result["description"], "A test parameter");
            assert_eq!(result["type"], "string");
            assert!(result.get("default").is_none());
        }

        #[test]
        fn test_to_mcp_tool_parameter_with_default() {
            let default_value = json!("default_string");
            let param = create_test_parameter(
                false,
                ToolParameterDefinition::String(Some(default_value.to_string())),
            );

            let result = param.to_mcp_tool_parameter();

            assert_eq!(result["description"], "A test parameter");
            assert_eq!(result["type"], "string");
            assert_eq!(result["default"], Value::String(default_value.to_string()));
        }

        #[test]
        fn test_get_value_with_provided_parameter() {
            let param = create_test_parameter(true, ToolParameterDefinition::String(None));

            let mut params = JsonObject::new();
            params.insert("test_param".to_string(), json!("provided_value"));

            let result = param.get_value(params).unwrap();

            assert_eq!(result, json!("provided_value"));
        }

        #[test]
        fn test_get_value_required_parameter_missing() {
            let param = create_test_parameter(true, ToolParameterDefinition::String(None));
            let params = JsonObject::new();

            let result = param.get_value(params);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Parameter test_param is required but not provided")
            );
        }

        #[test]
        fn test_get_value_optional_parameter_missing_with_default() {
            let default_value = json!("default_value");
            let param = create_test_parameter(
                false,
                ToolParameterDefinition::String(Some(default_value.to_string())),
            );
            let params = JsonObject::new();

            let result = param.get_value(params).unwrap();
            assert_eq!(result, Value::String(default_value.to_string()));
        }

        #[test]
        fn test_get_value_optional_parameter_missing_without_default() {
            let param = create_test_parameter(false, ToolParameterDefinition::String(None));
            let params = JsonObject::new();

            let result = param.get_value(params).unwrap();

            assert_eq!(result, Value::Null);
        }

        #[test]
        fn test_get_value_parameter_overrides_default() {
            let default_value = json!("default_value");
            let param = create_test_parameter(
                false,
                ToolParameterDefinition::String(Some(default_value.to_string())),
            );
            let mut params = JsonObject::new();
            params.insert("test_param".to_string(), json!("override_value"));

            let result = param.get_value(params).unwrap();

            assert_eq!(result, json!("override_value"));
        }

        #[test]
        fn test_parameter_with_different_types() {
            let test_cases = vec![
                (ToolParameterDefinition::Int(None), "integer"),
                (ToolParameterDefinition::Number(None), "number"),
                (ToolParameterDefinition::Boolean(None), "boolean"),
                (ToolParameterDefinition::String(None), "string"),
            ];

            for (definition, expected_type) in test_cases {
                let param = ToolParameter {
                    name: "test_param",
                    description: "Test parameter",
                    required: false,
                    definition,
                };

                let result = param.to_mcp_tool_parameter();
                assert_eq!(result["type"], expected_type);
            }
        }
    }
}
