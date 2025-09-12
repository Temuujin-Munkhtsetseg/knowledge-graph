use std::borrow::Cow;
use std::sync::Arc;

use database::querying::QueryingService;
use rmcp::model::{CallToolResult, Content, JsonObject, Tool, object};
use serde_json::json;
use workspace_manager::WorkspaceManager;

use crate::tools::get_references::constants::MIN_PAGE;
use crate::tools::get_references::constants::{
    DEFAULT_PAGE, DEFINITION_NAME_FIELD, FILE_PATH_FIELD, GET_REFERENCES_TOOL_DESCRIPTION,
    GET_REFERENCES_TOOL_NAME, PAGE_FIELD,
};
use crate::tools::get_references::input::GetReferencesToolInput;
use crate::tools::{get_references::service::GetReferencesService, types::KnowledgeGraphTool};

pub struct GetReferencesTool {
    workspace_manager: Arc<WorkspaceManager>,
    service: GetReferencesService,
}

impl GetReferencesTool {
    pub fn new(
        querying_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            workspace_manager: Arc::clone(&workspace_manager),
            service: GetReferencesService::new(querying_service),
        }
    }
}

impl KnowledgeGraphTool for GetReferencesTool {
    fn name(&self) -> &str {
        GET_REFERENCES_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let input_schema = json!({
            "type": "object",
            "properties": {
                DEFINITION_NAME_FIELD: {
                    "type": "string",
                    "description": "The exact identifier name to search. Must match the symbol name exactly as it appears in code, without namespace prefixes or file extensions. Example: 'myFunction', 'MyClass'."
                },
                FILE_PATH_FIELD: {
                    "type": "string",
                    "description": "Absolute or project-relative path to the file that contains the symbol usage. Example: src/main/java/com/example/User.java"
                },
                PAGE_FIELD: {
                    "type": "integer",
                    "description": "Page number starting from 1. If the response's next_page field is greater than 1, more results are available at that page. You can use this to retrieve more results if more context is needed.",
                    "default": DEFAULT_PAGE,
                    "minimum": MIN_PAGE,
                },
            },
            "required": [FILE_PATH_FIELD, DEFINITION_NAME_FIELD],
            "additionalProperties": false
        });

        Tool {
            name: Cow::Borrowed(GET_REFERENCES_TOOL_NAME),
            description: Some(Cow::Borrowed(GET_REFERENCES_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let input = GetReferencesToolInput::new(params, &self.workspace_manager)?;

        let output = self.service.get_references(input)?;

        Ok(CallToolResult::success(vec![
            Content::json(serde_json::to_value(output).unwrap()).unwrap(),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use database::{kuzu::database::KuzuDatabase, querying::DatabaseQueryingService};
    use indexer::analysis::languages::java::setup_java_reference_pipeline;
    use rmcp::model::object;
    use serde_json::json;

    use crate::tools::{get_references::tool::GetReferencesTool, types::KnowledgeGraphTool};

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_finds_method_call_references() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &GetReferencesTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        let project = &setup.workspace_manager.clone().list_all_projects()[0];

        // Test finding references to the "bar" method in Foo.java
        let result = tool
            .call(object(json!({
                "definition_name": "bar",
                "file_path": "main/src/com/example/app/Foo.java",
                "page": 1,
            })))
            .unwrap();

        let content = result.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content[0];
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        // Check the definitions and summary
        let definitions = parsed["definitions"].as_array().unwrap();
        let next_page = &parsed["next_page"];
        assert!(next_page.is_null());

        // Find the definition that contains the reference we're looking for
        let main_definition = definitions
            .iter()
            .find(|def| def["fqn"].as_str().unwrap() == "com.example.app.Main.main")
            .expect("Should find main method definition");

        // Check definition-level information
        assert_eq!(main_definition["name"].as_str().unwrap(), "main");
        assert_eq!(
            main_definition["location"].as_str().unwrap(),
            project.project_path.clone() + "/main/src/com/example/app/Main.java:L15-44"
        );
        assert_eq!(
            main_definition["definition_type"].as_str().unwrap(),
            "Method"
        );
        assert_eq!(
            main_definition["fqn"].as_str().unwrap(),
            "com.example.app.Main.main"
        );

        // Check the reference within this definition
        let references = main_definition["references"].as_array().unwrap();
        let first_ref = &references[0];
        assert_eq!(first_ref["reference_type"].as_str().unwrap(), "CALLS");
        assert_eq!(
            first_ref["location"].as_str().unwrap(),
            project.project_path.clone() + "/main/src/com/example/app/Main.java:L17-17"
        );
        assert_eq!(
            first_ref["context"].as_str().unwrap(),
            "@Traceable\n    public void main() {\n        if (this.myParameter.bar() instanceof Bar bar) {\n            bar.baz();\n        }"
        );

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_finds_class_constructor_references() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &GetReferencesTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        let project = &setup.workspace_manager.clone().list_all_projects()[0];

        // Test finding references to the "Foo" class constructor
        let result = tool
            .call(object(json!({
                "definition_name": "Bar",
                "file_path": project.project_path.clone() + "/main/src/com/example/app/Bar.java",
                "page": 1,
            })))
            .unwrap();

        let content = result.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content[0];
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        // Check the definitions and summary for Bar class
        let definitions = parsed["definitions"].as_array().unwrap();
        let next_page = &parsed["next_page"];
        assert!(next_page.is_null());

        // Find the definition that contains the reference we're looking for (Foo.bar method)
        let bar_definition = definitions
            .iter()
            .find(|def| def["fqn"].as_str().unwrap() == "com.example.app.Foo.bar")
            .expect("Should find bar method definition");

        // Check definition-level information
        assert_eq!(bar_definition["name"].as_str().unwrap(), "bar");
        assert_eq!(
            bar_definition["location"].as_str().unwrap(),
            project.project_path.clone() + "/main/src/com/example/app/Foo.java:L6-8"
        );
        assert_eq!(
            bar_definition["definition_type"].as_str().unwrap(),
            "Method"
        );
        assert_eq!(
            bar_definition["fqn"].as_str().unwrap(),
            "com.example.app.Foo.bar"
        );

        // Check the reference within this definition (constructor call)
        let references = bar_definition["references"].as_array().unwrap();
        let first_ref = &references[0];
        assert_eq!(first_ref["reference_type"].as_str().unwrap(), "CALLS");
        assert_eq!(
            first_ref["location"].as_str().unwrap(),
            project.project_path.clone() + "/main/src/com/example/app/Foo.java:L7-7"
        );
        assert_eq!(
            first_ref["context"].as_str().unwrap(),
            "public Bar bar() {\n        return new Bar();\n    }"
        );

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_pagination_limits_results_correctly() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &GetReferencesTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // First, get all results to see total count
        let result_all = tool
            .call(object(json!({
                "definition_name": "Foo",
                "file_path": "main/src/com/example/app/Foo.java",
                "page": 1,
            })))
            .unwrap();

        let content_all = result_all.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content_all[0];
        let json_str_all = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };
        let parsed_all: serde_json::Value =
            serde_json::from_str(json_str_all).expect("Expected valid JSON in content");

        assert_eq!(
            parsed_all["next_page"].as_u64().unwrap(),
            2,
            "Expected next_page to be present"
        );

        // Test pagination by limiting to 1 result
        let result_limited = tool
            .call(object(json!({
                "definition_name": "Foo",
                "file_path": "main/src/com/example/app/Foo.java",
                "page": 2,
            })))
            .unwrap();

        let content_limited = result_limited.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content_limited[0];
        let json_str_limited = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        let parsed_limited: serde_json::Value =
            serde_json::from_str(json_str_limited).expect("Expected valid JSON in content");
        let next_page = &parsed_limited["next_page"];
        assert!(next_page.is_null(), "Expected next_page to be null");

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_returns_empty_result_for_nonexistent_definition() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &GetReferencesTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        let project = &setup.workspace_manager.clone().list_all_projects()[0];

        // Test finding references to a non-existent method
        let result = tool
            .call(object(json!({
                "definition_name": "nonExistentMethod",
                "file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 1,
            })))
            .unwrap();

        let content = result.content.expect("Expected content in result");
        assert!(!content.is_empty(), "Expected non-empty content");

        let rmcp::model::Annotated { raw, .. } = &content[0];
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        let definitions = parsed["definitions"].as_array().unwrap();
        let next_page = &parsed["next_page"];

        assert_eq!(
            definitions.len(),
            0,
            "Expected no definitions for non-existent method"
        );
        assert!(next_page.is_null(), "Expected next_page to be null");

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_returns_error_for_file_not_in_workspace() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &GetReferencesTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // Test with a file that doesn't exist in the project
        let result = tool.call(object(json!({
            "definition_name": "bar",
            "file_path": "non/existent/path/NonExistent.java",
            "page": 1,
        })));

        // This should return an error since the file is not in the workspace
        assert!(result.is_err(), "Expected error for invalid file path");

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_handles_invalid_parameter_values_gracefully() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &GetReferencesTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        let result_large_page = tool.call(object(json!({
            "definition_name": "bar",
            "file_path": "main/src/com/example/app/Foo.java",
            "page": 999999,
        })));
        assert!(
            result_large_page.is_ok(),
            "Should handle oversized limit gracefully"
        );

        let result_zero_page = tool.call(object(json!({
            "definition_name": "bar",
            "file_path": "main/src/com/example/app/Foo.java",
            "page": 0,
        })));
        assert!(
            result_zero_page.is_ok(),
            "Should handle zero limit gracefully"
        );

        let result_negative_page = tool.call(object(json!({
            "definition_name": "bar",
            "file_path": "main/src/com/example/app/Foo.java",
            "page": -10,
        })));
        assert!(
            result_negative_page.is_ok(),
            "Should handle negative offset gracefully"
        );

        let result_missing_definition = tool.call(object(json!({
            "file_path": "main/src/com/example/app/Foo.java",
            "page": 1,
        })));
        assert!(
            result_missing_definition.is_err(),
            "Should return error for missing definition_name"
        );

        let result_missing_file_path = tool.call(object(json!({
            "definition_name": "bar",
            "page": 1,
        })));
        assert!(
            result_missing_file_path.is_err(),
            "Should return error for missing absolute_file_path"
        );

        let result_empty_definition = tool.call(object(json!({
            "definition_name": "",
            "file_path": "main/src/com/example/app/Foo.java",
            "page": 1,
        })));
        assert!(
            result_empty_definition.is_err(),
            "Should return error for empty definition_name"
        );

        setup.cleanup();
    }
}
