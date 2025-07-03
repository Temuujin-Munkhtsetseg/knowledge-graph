use crate::tools::types::{KnowledgeGraphTool, ToolParameter, ToolParameterKind};
use database::querying::{Query, QueryParameter, QueryingService};
use rmcp::model::{CallToolResult, Content, JsonObject, Tool};
use serde_json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

pub struct QueryKnowledgeGraphTool {
    query_service: Arc<dyn QueryingService>,
    name: &'static str,
    query: &'static str,
    description: &'static str,
    parameters: HashMap<&'static str, ToolParameter>,
}

impl QueryKnowledgeGraphTool {
    pub fn new(query_service: Arc<dyn QueryingService>, query: Query) -> Self {
        Self {
            query_service,
            name: query.slug,
            query: query.query,
            description: query.description,
            parameters: extract_parameters(query.parameters),
        }
    }
}

const PROJECT_PARAMETER: ToolParameter = ToolParameter {
    name: "project",
    description: "The project to execute the query on. This is the path to current project directory.",
    required: true,
    kind: ToolParameterKind::String,
    default: None,
};

impl KnowledgeGraphTool for QueryKnowledgeGraphTool {
    fn name(&self) -> &str {
        self.name
    }

    fn to_mcp_tool(&self) -> Tool {
        let (properties, required) = self.parameters.iter().fold(
            (JsonObject::new(), Vec::new()),
            |(mut properties, mut required), (name, parameter)| {
                properties.insert(name.to_string(), parameter.to_mcp_tool_parameter());

                if parameter.required {
                    required.push(name.to_string());
                }

                (properties, required)
            },
        );

        let mut input_schema = JsonObject::new();
        input_schema.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        input_schema.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        input_schema.insert(
            "required".to_string(),
            serde_json::Value::Array(
                required
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );

        Tool {
            name: Cow::Borrowed(self.name),
            description: Cow::Borrowed(self.description),
            input_schema: Arc::new(input_schema),
        }
    }

    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::Error> {
        let mut query_params = JsonObject::with_capacity(self.parameters.len());

        for (name, parameter) in &self.parameters {
            let value = parameter.get_value(params.clone()).map_err(|e| {
                rmcp::Error::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    format!("Could not get value for parameter {name}: {e}"),
                    None,
                )
            })?;

            query_params.insert(name.to_string(), value);
        }

        let project_path = query_params.remove(PROJECT_PARAMETER.name);
        if project_path.is_none() {
            return Err(rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                "Parameter 'project' is required to create a query but not provided.",
                None,
            ));
        }

        let mut result = self
            .query_service
            .execute_query(
                project_path.unwrap().as_str().unwrap(),
                self.query,
                query_params,
            )
            .map_err(|e| {
                rmcp::Error::new(
                    rmcp::model::ErrorCode::INVALID_REQUEST,
                    format!("Could not execute query: {e}."),
                    None,
                )
            })?;

        let json_result = result.to_json().map_err(|e| {
            rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Could not convert query result to JSON: {e}."),
                None,
            )
        })?;

        Ok(CallToolResult::success(vec![
            Content::json(json_result).unwrap(),
        ]))
    }
}

fn extract_parameters(
    parameters: HashMap<&'static str, QueryParameter>,
) -> HashMap<&'static str, ToolParameter> {
    let mut result = HashMap::with_capacity(parameters.len());

    for (name, parameter) in parameters {
        result.insert(
            name,
            ToolParameter {
                name,
                description: parameter.description,
                required: parameter.required,
                kind: ToolParameterKind::from_query_kind(parameter.kind),
                default: parameter.default,
            },
        );
    }

    result.insert(PROJECT_PARAMETER.name, PROJECT_PARAMETER);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::{
        querying::library::{Query, QueryParameter, QueryParameterKind},
        querying::testing::MockQueryingService,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn create_test_query() -> Query {
        let mut parameters = HashMap::new();
        parameters.insert(
            "fqn",
            QueryParameter {
                name: "fqn",
                description: "The fully qualified name",
                required: true,
                kind: QueryParameterKind::String,
                default: None,
            },
        );
        parameters.insert(
            "limit",
            QueryParameter {
                name: "limit",
                description: "Maximum number of results",
                required: false,
                kind: QueryParameterKind::Int,
                default: Some(json!(10)),
            },
        );

        Query {
            name: "Test Query",
            slug: "test_query",
            description: "A test query for testing purposes",
            query: "MATCH (n) WHERE n.fqn = $fqn RETURN n LIMIT $limit",
            parameters,
        }
    }

    mod constructor_tests {
        use super::*;

        #[test]
        fn test_new_creates_tool_with_correct_properties() {
            let mock_service = Arc::new(MockQueryingService::new());
            let query = create_test_query();

            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            assert_eq!(tool.name, "test_query");
            assert_eq!(
                tool.query,
                "MATCH (n) WHERE n.fqn = $fqn RETURN n LIMIT $limit"
            );
            assert_eq!(tool.description, "A test query for testing purposes");
        }

        #[test]
        fn test_new_extracts_parameters_and_include_project_parameter_correctly() {
            let mock_service = Arc::new(MockQueryingService::new());
            let query = create_test_query();

            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            assert_eq!(tool.parameters.len(), 3);
            assert!(tool.parameters.contains_key("fqn"));
            assert!(tool.parameters.contains_key("limit"));
            assert!(tool.parameters.contains_key("project"));
        }
    }

    mod to_mcp_tool_tests {
        use super::*;

        #[test]
        fn test_to_mcp_tool_includes_all_parameters() {
            let mock_service = Arc::new(MockQueryingService::new());
            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let mcp_tool = tool.to_mcp_tool();
            let schema = mcp_tool.input_schema.as_ref();
            let properties = schema.get("properties").unwrap().as_object().unwrap();
            let required = schema.get("required").unwrap().as_array().unwrap();

            assert_eq!(properties.len(), 3);
            assert!(properties.contains_key("fqn"));
            assert!(properties.contains_key("limit"));
            assert!(properties.contains_key("project"));

            assert_eq!(required.len(), 2);
            assert!(required.contains(&json!("fqn")));
            assert!(required.contains(&json!("project")));
            assert!(!required.contains(&json!("limit"))); // limit is optional
        }

        #[test]
        fn test_to_mcp_tool_parameter_with_default_value() {
            let mock_service = Arc::new(MockQueryingService::new());
            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let mcp_tool = tool.to_mcp_tool();
            let schema = mcp_tool.input_schema.as_ref();
            let properties = schema.get("properties").unwrap().as_object().unwrap();

            let limit_property = properties.get("limit").unwrap().as_object().unwrap();
            assert_eq!(limit_property.get("type").unwrap(), "int");
            assert_eq!(limit_property.get("default").unwrap(), &json!(10));
        }
    }

    mod call_method_tests {
        use super::*;

        #[test]
        fn test_call_executes_query_successfully() {
            let expected_params = {
                let mut params = serde_json::Map::new();
                params.insert("fqn".to_string(), json!("test.fqn"));
                params.insert("limit".to_string(), json!(5));
                params
            };

            let mock_service = Arc::new(
                MockQueryingService::new()
                    .with_expectations(
                        "/test/project".to_string(),
                        "MATCH (n) WHERE n.fqn = $fqn RETURN n LIMIT $limit".to_string(),
                        expected_params,
                    )
                    .with_return_data(
                        vec!["name".to_string(), "type".to_string()],
                        vec![
                            vec!["TestClass".to_string(), "class".to_string()],
                            vec!["TestMethod".to_string(), "method".to_string()],
                        ],
                    ),
            );

            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let params = json!({
                "project": "/test/project",
                "fqn": "test.fqn",
                "limit": 5
            })
            .as_object()
            .unwrap()
            .clone();

            let result = tool.call(params).unwrap();

            assert!(!result.is_error.unwrap());
            assert_eq!(result.content.len(), 1);

            // The content should be JSON data
            let content = &result.content[0];
            if let Ok(json_data) =
                serde_json::from_str::<serde_json::Value>(&content.as_text().unwrap().text)
            {
                if let Some(data_array) = json_data.as_array() {
                    println!("data_array: {data_array:?}");
                    assert_eq!(data_array.len(), 2);
                    assert_eq!(data_array[0]["name"], "TestClass");
                    assert_eq!(data_array[0]["type"], "class");
                    assert_eq!(data_array[1]["name"], "TestMethod");
                    assert_eq!(data_array[1]["type"], "method");
                } else {
                    panic!("Expected JSON array data");
                }
            } else {
                panic!("Expected JSON content");
            }
        }

        #[test]
        fn test_call_fails_when_project_parameter_missing() {
            let mock_service = Arc::new(MockQueryingService::new());
            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let params = json!({
                "fqn": "test.fqn",
                "limit": 5
            })
            .as_object()
            .unwrap()
            .clone();

            let result = tool.call(params);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
        }

        #[test]
        fn test_call_uses_default_values_for_optional_parameters() {
            let expected_params = {
                let mut params = serde_json::Map::new();
                params.insert("fqn".to_string(), json!("test.fqn"));
                params.insert("limit".to_string(), json!(10)); // default value
                params
            };

            let mock_service = Arc::new(MockQueryingService::new().with_expectations(
                "/test/project".to_string(),
                "MATCH (n) WHERE n.fqn = $fqn RETURN n LIMIT $limit".to_string(),
                expected_params,
            ));

            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let params = json!({
                "project": "/test/project",
                "fqn": "test.fqn"
                // limit not provided, should use default
            })
            .as_object()
            .unwrap()
            .clone();

            let result = tool.call(params);
            assert!(result.is_ok());
        }

        #[test]
        fn test_call_fails_when_required_parameter_missing() {
            let mock_service = Arc::new(MockQueryingService::new());
            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let params = json!({
                "project": "/test/project"
                // fqn is required but missing
            })
            .as_object()
            .unwrap()
            .clone();

            let result = tool.call(params);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
        }

        #[test]
        fn test_call_handles_query_service_failure() {
            let mock_service = Arc::new(MockQueryingService::new().with_failure());
            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let params = json!({
                "project": "/test/project",
                "fqn": "test.fqn"
            })
            .as_object()
            .unwrap()
            .clone();

            let result = tool.call(params);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_REQUEST);
        }

        #[test]
        fn test_call_removes_project_parameter_from_database_query_params() {
            let expected_params = {
                let mut params = serde_json::Map::new();
                params.insert("fqn".to_string(), json!("test.fqn"));
                params.insert("limit".to_string(), json!(10));
                params
            };

            let mock_service = Arc::new(MockQueryingService::new().with_expectations(
                "/test/project".to_string(),
                "MATCH (n) WHERE n.fqn = $fqn RETURN n LIMIT $limit".to_string(),
                expected_params,
            ));

            let query = create_test_query();
            let tool = QueryKnowledgeGraphTool::new(mock_service, query);

            let params = json!({
                "project": "/test/project",
                "fqn": "test.fqn"
            })
            .as_object()
            .unwrap()
            .clone();

            let result = tool.call(params);
            assert!(result.is_ok());
        }
    }
}
