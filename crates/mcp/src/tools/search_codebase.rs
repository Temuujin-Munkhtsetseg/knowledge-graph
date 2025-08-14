use std::{borrow::Cow, sync::Arc};

use database::querying::QueryLibrary;
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, Tool};
use serde_json::{Map, Value};

use crate::tools::types::KnowledgeGraphTool;
use workspace_manager::WorkspaceManager;

pub const SEARCH_CODEBASE_TOOL_NAME: &str = "search_codebase";
const SEARCH_CODEBASE_TOOL_DESCRIPTION: &str = "Searches for specific text, functions, variables, or code across all files in the codebase. \
Use this to locate specific implementations, track dependencies, find usage examples, or identify all occurrences of a particular element.";

pub struct SearchCodebaseTool {
    pub query_service: Arc<dyn database::querying::QueryingService>,
    pub workspace_manager: Arc<WorkspaceManager>,
}

impl SearchCodebaseTool {
    pub fn new(
        query_service: Arc<dyn database::querying::QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            query_service,
            workspace_manager,
        }
    }
}

impl KnowledgeGraphTool for SearchCodebaseTool {
    fn name(&self) -> &str {
        SEARCH_CODEBASE_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let mut properties = JsonObject::new();

        let mut project_property = JsonObject::new();
        project_property.insert("type".to_string(), Value::String("string".to_string()));
        project_property.insert(
            "description".to_string(),
            Value::String("The absolutepath to current project directory.".to_string()),
        );
        properties.insert(
            "project_absolute_path".to_string(),
            Value::Object(project_property),
        );

        let mut search_term_property = JsonObject::new();

        let mut search_term_items = JsonObject::new();
        search_term_items.insert("type".to_string(), Value::String("string".to_string()));

        search_term_property.insert("items".to_string(), Value::Object(search_term_items));
        search_term_property.insert("type".to_string(), Value::String("array".to_string()));
        search_term_property.insert(
            "description".to_string(),
            Value::String(
                "The texts, function names, variable names, or patterns to search for".to_string(),
            ),
        );
        properties.insert(
            "search_terms".to_string(),
            Value::Object(search_term_property),
        );

        let mut limit_property = JsonObject::new();
        limit_property.insert("type".to_string(), Value::String("number".to_string()));
        limit_property.insert(
            "description".to_string(),
            Value::String("The maximum number of results to return".to_string()),
        );
        limit_property.insert("default".to_string(), Value::Number(50.into()));
        properties.insert("limit".to_string(), Value::Object(limit_property));

        let mut input_schema = JsonObject::new();
        input_schema.insert("type".to_string(), Value::String("object".to_string()));
        input_schema.insert("properties".to_string(), Value::Object(properties));
        input_schema.insert(
            "required".to_string(),
            Value::Array(vec![
                Value::String("search_terms".to_string()),
                Value::String("project_absolute_path".to_string()),
            ]),
        );

        Tool {
            name: Cow::Borrowed(SEARCH_CODEBASE_TOOL_NAME),
            description: Some(Cow::Borrowed(SEARCH_CODEBASE_TOOL_DESCRIPTION)),
            input_schema: Arc::new(input_schema),
            output_schema: None,
            annotations: None,
        }
    }

    fn call(
        &self,
        params: rmcp::model::JsonObject,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let search_terms = params.get("search_terms").and_then(|v| v.as_array());
        if search_terms.is_none() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Missing search_terms".to_string(),
                None,
            ));
        }

        let project_absolute_path = params.get("project_absolute_path").and_then(|v| v.as_str());
        if project_absolute_path.is_none() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Missing project_absolute_path".to_string(),
                None,
            ));
        }
        let project_absolute_path = project_absolute_path.unwrap();

        let database_path = self
            .workspace_manager
            .get_project_for_path(project_absolute_path)
            .map(|p| p.database_path);
        if database_path.is_none() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Project not found in workspace manager".to_string(),
                None,
            ));
        }
        let database_path = database_path.unwrap();

        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50);

        let search_terms = search_terms.unwrap();
        let mut results = Vec::new();

        for search_term in search_terms {
            if results.len() >= limit as usize {
                break;
            }

            let query = QueryLibrary::get_search_nodes_query();
            let mut query_params = Map::new();

            query_params.insert("search_term".to_string(), search_term.clone());
            query_params.insert(
                "limit".to_string(),
                Value::Number((limit - results.len() as u64).into()),
            );

            let mut query_result = self
                .query_service
                .execute_query(database_path.clone(), query.query, query_params)
                .map_err(|e| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_REQUEST,
                        format!("Could not execute query: {e}."),
                        None,
                    )
                })?;

            let result = query_result.to_json(&query.result).map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    format!("Could not convert query result to JSON: {e}."),
                    None,
                )
            })?;

            results.push(result);
        }

        let mut content = Vec::new();
        for item in results {
            content.push(Content::json(item).unwrap());
        }

        Ok(CallToolResult::success(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::testing::MockQueryingService;
    use std::sync::Arc;
    use tempfile::TempDir;
    use testing::repository::TestRepository;

    fn create_test_workspace_manager() -> (Arc<WorkspaceManager>, String) {
        let temp_workspace_dir = TempDir::new().unwrap();
        let workspace_path = temp_workspace_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let test_project_path = workspace_path.join("test_project");
        TestRepository::new(&test_project_path, Some("test-repo"));

        let temp_data_dir = TempDir::new().unwrap();
        let manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );

        manager.register_workspace_folder(&workspace_path).unwrap();

        let projects = manager.list_all_projects();
        let project_path = projects[0].project_path.clone();

        (manager, project_path)
    }

    #[test]
    fn test_search_codebase_with_multiple_search_terms() {
        let search_results_1 = vec![vec![
            "1".to_string(),
            "DefinitionNode".to_string(),
            "UserModel".to_string(),
            "app/models/user_model.rb".to_string(),
            "".to_string(),
            "".to_string(),
            "app.models.user_model.UserModel".to_string(),
            "class".to_string(),
            "".to_string(),
            "".to_string(),
            "4".to_string(),
            "100".to_string(),
            "200".to_string(),
            "1".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]];

        let search_results_2 = vec![vec![
            "2".to_string(),
            "FileNode".to_string(),
            "base_model.rb".to_string(),
            "app/models/base_model.rb".to_string(),
            "/project/app/models/base_model.rb".to_string(),
            "test-repo".to_string(),
            "".to_string(),
            "".to_string(),
            "ruby".to_string(),
            "rb".to_string(),
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]];

        let column_names = vec![
            "id".to_string(),
            "node_type".to_string(),
            "name".to_string(),
            "path".to_string(),
            "absolute_path".to_string(),
            "repository_name".to_string(),
            "fqn".to_string(),
            "definition_type".to_string(),
            "language".to_string(),
            "extension".to_string(),
            "start_line".to_string(),
            "primary_start_byte".to_string(),
            "primary_end_byte".to_string(),
            "total_locations".to_string(),
            "import_type".to_string(),
            "import_path".to_string(),
            "import_alias".to_string(),
        ];

        let mock_query_service = MockQueryingService::new()
            .with_return_data(column_names.clone(), search_results_2)
            .with_return_data(column_names, search_results_1);

        let (workspace_manager, project_path) = create_test_workspace_manager();
        let tool = SearchCodebaseTool::new(Arc::new(mock_query_service), workspace_manager);

        let mut params = JsonObject::new();
        params.insert(
            "project_absolute_path".to_string(),
            Value::String(project_path),
        );
        params.insert(
            "search_terms".to_string(),
            Value::Array(vec![
                Value::String("UserModel".to_string()),
                Value::String("BaseModel".to_string()),
            ]),
        );
        params.insert("limit".to_string(), Value::Number(100.into()));

        let result = tool.call(params).unwrap();
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert_eq!(content.len(), 2); // Two search terms, two results

        // Verify first result contains UserModel
        let first_result = &content[0];
        let first_json: Value =
            serde_json::from_str(&first_result.raw.as_text().unwrap().text).unwrap();
        let first_array = first_json.as_array().unwrap();
        assert_eq!(first_array.len(), 1);
        let first_item = &first_array[0];
        assert_eq!(first_item["name"], "UserModel");
        assert_eq!(first_item["node_type"], "DefinitionNode");

        // Verify second result contains BaseModel
        let second_result = &content[1];
        let second_json: Value =
            serde_json::from_str(&second_result.raw.as_text().unwrap().text).unwrap();
        let second_array = second_json.as_array().unwrap();
        assert_eq!(second_array.len(), 1);
        let second_item = &second_array[0];
        assert_eq!(second_item["name"], "base_model.rb");
        assert_eq!(second_item["node_type"], "FileNode");
    }

    #[test]
    fn test_search_codebase_with_single_search_term() {
        let search_results = vec![
            vec![
                "1".to_string(),
                "DefinitionNode".to_string(),
                "UserModel".to_string(),
                "app/models/user_model.rb".to_string(),
                "".to_string(),
                "".to_string(),
                "app.models.user_model.UserModel".to_string(),
                "class".to_string(),
                "".to_string(),
                "".to_string(),
                "4".to_string(),
                "100".to_string(),
                "200".to_string(),
                "1".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
            vec![
                "2".to_string(),
                "DefinitionNode".to_string(),
                "full_name".to_string(),
                "app/models/user_model.rb".to_string(),
                "".to_string(),
                "".to_string(),
                "app.models.user_model.UserModel.full_name".to_string(),
                "method".to_string(),
                "".to_string(),
                "".to_string(),
                "5".to_string(),
                "150".to_string(),
                "180".to_string(),
                "1".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
        ];

        let column_names = vec![
            "id".to_string(),
            "node_type".to_string(),
            "name".to_string(),
            "path".to_string(),
            "absolute_path".to_string(),
            "repository_name".to_string(),
            "fqn".to_string(),
            "definition_type".to_string(),
            "language".to_string(),
            "extension".to_string(),
            "start_line".to_string(),
            "primary_start_byte".to_string(),
            "primary_end_byte".to_string(),
            "total_locations".to_string(),
            "import_type".to_string(),
            "import_path".to_string(),
            "import_alias".to_string(),
        ];

        let mock_query_service =
            MockQueryingService::new().with_return_data(column_names, search_results);

        let (workspace_manager, project_path) = create_test_workspace_manager();
        let tool = SearchCodebaseTool::new(Arc::new(mock_query_service), workspace_manager);

        let mut params = JsonObject::new();
        params.insert(
            "project_absolute_path".to_string(),
            Value::String(project_path),
        );
        params.insert(
            "search_terms".to_string(),
            Value::Array(vec![Value::String("UserModel".to_string())]),
        );

        let result = tool.call(params).unwrap();
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert_eq!(content.len(), 1); // One search term, one result

        let first_result = &content[0];
        let first_json: Value =
            serde_json::from_str(&first_result.raw.as_text().unwrap().text).unwrap();
        let first_array = first_json.as_array().unwrap();
        assert_eq!(first_array.len(), 2); // Two matching nodes found

        // Verify first match
        let first_item = &first_array[0];
        assert_eq!(first_item["name"], "UserModel");
        assert_eq!(first_item["node_type"], "DefinitionNode");
        assert_eq!(first_item["definition_type"], "class");

        // Verify second match
        let second_item = &first_array[1];
        assert_eq!(second_item["name"], "full_name");
        assert_eq!(second_item["node_type"], "DefinitionNode");
        assert_eq!(second_item["definition_type"], "method");
    }

    #[test]
    fn test_search_codebase_with_no_search_terms() {
        let mock_query_service = MockQueryingService::new();
        let (workspace_manager, project_path) = create_test_workspace_manager();
        let tool = SearchCodebaseTool::new(Arc::new(mock_query_service), workspace_manager);

        let mut params = JsonObject::new();
        params.insert(
            "project_absolute_path".to_string(),
            Value::String(project_path),
        );
        params.insert("search_terms".to_string(), Value::Array(vec![]));

        let result = tool.call(params).unwrap();
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert_eq!(content.len(), 0); // No search terms, no results
    }

    #[test]
    fn test_search_codebase_hits_limit() {
        // Create search results - first query returns 1 result, second would return 2 more
        let search_results_1 = vec![vec![
            "1".to_string(),
            "DefinitionNode".to_string(),
            "UserModel1".to_string(),
            "app/models/user_model1.rb".to_string(),
            "".to_string(),
            "".to_string(),
            "app.models.user_model1.UserModel1".to_string(),
            "class".to_string(),
            "".to_string(),
            "".to_string(),
            "4".to_string(),
            "100".to_string(),
            "200".to_string(),
            "1".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]];

        let search_results_2 = vec![
            vec![
                "2".to_string(),
                "DefinitionNode".to_string(),
                "AnotherModel1".to_string(),
                "app/models/another_model1.rb".to_string(),
                "".to_string(),
                "".to_string(),
                "app.models.another_model1.AnotherModel1".to_string(),
                "class".to_string(),
                "".to_string(),
                "".to_string(),
                "5".to_string(),
                "300".to_string(),
                "400".to_string(),
                "1".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
            vec![
                "3".to_string(),
                "DefinitionNode".to_string(),
                "AnotherModel2".to_string(),
                "app/models/another_model2.rb".to_string(),
                "".to_string(),
                "".to_string(),
                "app.models.another_model2.AnotherModel2".to_string(),
                "class".to_string(),
                "".to_string(),
                "".to_string(),
                "6".to_string(),
                "500".to_string(),
                "600".to_string(),
                "1".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
        ];

        let column_names = vec![
            "id".to_string(),
            "node_type".to_string(),
            "name".to_string(),
            "path".to_string(),
            "absolute_path".to_string(),
            "repository_name".to_string(),
            "fqn".to_string(),
            "definition_type".to_string(),
            "language".to_string(),
            "extension".to_string(),
            "start_line".to_string(),
            "primary_start_byte".to_string(),
            "primary_end_byte".to_string(),
            "total_locations".to_string(),
            "import_type".to_string(),
            "import_path".to_string(),
            "import_alias".to_string(),
        ];

        let mock_query_service = MockQueryingService::new()
            .with_return_data(column_names.clone(), search_results_2)
            .with_return_data(column_names, search_results_1);

        let (workspace_manager, project_path) = create_test_workspace_manager();
        let tool = SearchCodebaseTool::new(Arc::new(mock_query_service), workspace_manager);

        let mut params = JsonObject::new();
        params.insert(
            "project_absolute_path".to_string(),
            Value::String(project_path),
        );
        params.insert(
            "search_terms".to_string(),
            Value::Array(vec![
                Value::String("UserModel".to_string()),
                Value::String("AnotherModel".to_string()),
                Value::String("ThirdModel".to_string()),
            ]),
        );
        params.insert("limit".to_string(), Value::Number(1.into())); // Limit to 1 search term

        let result = tool.call(params).unwrap();
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert_eq!(content.len(), 1); // Should process only one search term due to limit

        // Verify the first result
        let first_result = &content[0];
        let first_json: Value =
            serde_json::from_str(&first_result.raw.as_text().unwrap().text).unwrap();
        let first_array = first_json.as_array().unwrap();
        assert_eq!(first_array.len(), 1); // One result from first search term
        assert_eq!(first_array[0]["name"], "UserModel1");
    }
}
