use anyhow::Error;
use querying::QueryParameterKind;
use rmcp::model::CallToolResult;
use rmcp::model::{JsonObject, Tool};
use serde_json::Value;

pub trait KnowledgeGraphTool: Send + Sync {
    fn name(&self) -> &str;
    fn to_mcp_tool(&self) -> Tool;
    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::Error>;
}

pub enum ToolParameterKind {
    String,
    Int,
    Float,
    Boolean,
    StringList,
    IntList,
    FloatList,
}

impl ToolParameterKind {
    pub fn from_query_kind(kind: QueryParameterKind) -> ToolParameterKind {
        match kind {
            QueryParameterKind::String => ToolParameterKind::String,
            QueryParameterKind::Int => ToolParameterKind::Int,
            QueryParameterKind::Float => ToolParameterKind::Float,
            QueryParameterKind::Boolean => ToolParameterKind::Boolean,
            QueryParameterKind::StringList => ToolParameterKind::StringList,
            QueryParameterKind::IntList => ToolParameterKind::IntList,
            QueryParameterKind::FloatList => ToolParameterKind::FloatList,
        }
    }

    pub fn to_mcp_tool_type(&self) -> String {
        match self {
            ToolParameterKind::String => "string".to_string(),
            ToolParameterKind::Int => "int".to_string(),
            ToolParameterKind::Float => "float".to_string(),
            ToolParameterKind::Boolean => "boolean".to_string(),
            ToolParameterKind::StringList => "string[]".to_string(),
            ToolParameterKind::IntList => "int[]".to_string(),
            ToolParameterKind::FloatList => "float[]".to_string(),
        }
    }
}

pub struct ToolParameter {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub kind: ToolParameterKind,
    pub default: Option<Value>,
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
            serde_json::Value::String(self.kind.to_mcp_tool_type()),
        );

        if self.default.is_some() {
            fields.insert("default".to_string(), self.default.clone().unwrap());
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

            return Ok(self.default.clone().unwrap_or(serde_json::Value::Null));
        }

        Ok(value.unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod tool_parameter_kind_tests {
        use super::*;

        #[test]
        fn test_from_query_kind_conversion() {
            assert!(matches!(
                ToolParameterKind::from_query_kind(QueryParameterKind::String),
                ToolParameterKind::String
            ));
            assert!(matches!(
                ToolParameterKind::from_query_kind(QueryParameterKind::Int),
                ToolParameterKind::Int
            ));
            assert!(matches!(
                ToolParameterKind::from_query_kind(QueryParameterKind::Float),
                ToolParameterKind::Float
            ));
            assert!(matches!(
                ToolParameterKind::from_query_kind(QueryParameterKind::Boolean),
                ToolParameterKind::Boolean
            ));
            assert!(matches!(
                ToolParameterKind::from_query_kind(QueryParameterKind::StringList),
                ToolParameterKind::StringList
            ));
            assert!(matches!(
                ToolParameterKind::from_query_kind(QueryParameterKind::IntList),
                ToolParameterKind::IntList
            ));
            assert!(matches!(
                ToolParameterKind::from_query_kind(QueryParameterKind::FloatList),
                ToolParameterKind::FloatList
            ));
        }

        #[test]
        fn test_to_mcp_tool_type_mapping() {
            assert_eq!(ToolParameterKind::String.to_mcp_tool_type(), "string");
            assert_eq!(ToolParameterKind::Int.to_mcp_tool_type(), "int");
            assert_eq!(ToolParameterKind::Float.to_mcp_tool_type(), "float");
            assert_eq!(ToolParameterKind::Boolean.to_mcp_tool_type(), "boolean");
            assert_eq!(ToolParameterKind::StringList.to_mcp_tool_type(), "string[]");
            assert_eq!(ToolParameterKind::IntList.to_mcp_tool_type(), "int[]");
            assert_eq!(ToolParameterKind::FloatList.to_mcp_tool_type(), "float[]");
        }
    }

    mod tool_parameter_tests {
        use super::*;

        fn create_test_parameter(required: bool, default: Option<Value>) -> ToolParameter {
            ToolParameter {
                name: "test_param",
                description: "A test parameter",
                required,
                kind: ToolParameterKind::String,
                default,
            }
        }

        #[test]
        fn test_to_mcp_tool_parameter_without_default() {
            let param = create_test_parameter(true, None);

            let result = param.to_mcp_tool_parameter();

            assert_eq!(result["description"], "A test parameter");
            assert_eq!(result["type"], "string");
            assert!(result.get("default").is_none());
        }

        #[test]
        fn test_to_mcp_tool_parameter_with_default() {
            let default_value = json!("default_string");
            let param = create_test_parameter(false, Some(default_value.clone()));

            let result = param.to_mcp_tool_parameter();

            assert_eq!(result["description"], "A test parameter");
            assert_eq!(result["type"], "string");
            assert_eq!(result["default"], default_value);
        }

        #[test]
        fn test_get_value_with_provided_parameter() {
            let param = create_test_parameter(true, None);

            let mut params = JsonObject::new();
            params.insert("test_param".to_string(), json!("provided_value"));

            let result = param.get_value(params).unwrap();

            assert_eq!(result, json!("provided_value"));
        }

        #[test]
        fn test_get_value_required_parameter_missing() {
            let param = create_test_parameter(true, None);
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
            let param = create_test_parameter(false, Some(default_value.clone()));
            let params = JsonObject::new();

            let result = param.get_value(params).unwrap();

            assert_eq!(result, default_value);
        }

        #[test]
        fn test_get_value_optional_parameter_missing_without_default() {
            let param = create_test_parameter(false, None);
            let params = JsonObject::new();

            let result = param.get_value(params).unwrap();

            assert_eq!(result, Value::Null);
        }

        #[test]
        fn test_get_value_parameter_overrides_default() {
            let default_value = json!("default_value");
            let param = create_test_parameter(false, Some(default_value));
            let mut params = JsonObject::new();
            params.insert("test_param".to_string(), json!("override_value"));

            let result = param.get_value(params).unwrap();

            assert_eq!(result, json!("override_value"));
        }

        #[test]
        fn test_parameter_with_different_types() {
            let test_cases = vec![
                (ToolParameterKind::Int, "int"),
                (ToolParameterKind::Float, "float"),
                (ToolParameterKind::Boolean, "boolean"),
                (ToolParameterKind::StringList, "string[]"),
                (ToolParameterKind::IntList, "int[]"),
                (ToolParameterKind::FloatList, "float[]"),
            ];

            for (kind, expected_type) in test_cases {
                let param = ToolParameter {
                    name: "test_param",
                    description: "Test parameter",
                    required: false,
                    kind,
                    default: None,
                };

                let result = param.to_mcp_tool_parameter();
                assert_eq!(result["type"], expected_type);
            }
        }
    }
}
