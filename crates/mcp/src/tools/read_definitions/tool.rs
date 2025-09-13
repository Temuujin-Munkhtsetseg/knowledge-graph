use std::borrow::Cow;
use std::sync::Arc;

use database::querying::QueryingService;
use rmcp::model::{CallToolResult, Content, JsonObject, Tool, object};
use serde_json::json;
use workspace_manager::WorkspaceManager;

use crate::tools::read_definitions::constants::{
    DEFINITIONS_FIELD, FILE_PATH_FIELD, NAMES_FIELD, READ_DEFINITIONS_TOOL_DESCRIPTION,
    READ_DEFINITIONS_TOOL_NAME,
};
use crate::tools::read_definitions::input::ReadDefinitionsToolInput;
use crate::tools::{read_definitions::service::ReadDefinitionsService, types::KnowledgeGraphTool};

pub struct ReadDefinitionsTool {
    workspace_manager: Arc<WorkspaceManager>,
    service: ReadDefinitionsService,
}

impl ReadDefinitionsTool {
    pub fn new(
        querying_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            workspace_manager: Arc::clone(&workspace_manager),
            service: ReadDefinitionsService::new(querying_service),
        }
    }
}

impl KnowledgeGraphTool for ReadDefinitionsTool {
    fn name(&self) -> &str {
        READ_DEFINITIONS_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let input_schema = json!({
            "type": "object",
            "properties": {
                DEFINITIONS_FIELD: {
                    "type": "array",
                    "description": "Array of definition requests with names array and file path.",
                    "items": {
                        "type": "object",
                        "properties": {
                            NAMES_FIELD: {
                                "type": "array",
                                "description": "Array of exact identifier names to read from the same file. Must match symbol names exactly as they appear in code, without namespace prefixes or file extensions. Example: ['myFunction', 'MyClass'].",
                                "items": {
                                    "type": "string"
                                },
                                "minItems": 1
                            },
                            FILE_PATH_FIELD: {
                                "type": "string",
                                "description": "Absolute or project-relative path to the file that contains the definitions. Example: src/main/java/com/example/User.java"
                            }
                        },
                        "required": [NAMES_FIELD, FILE_PATH_FIELD],
                        "additionalProperties": false
                    },
                }
            },
            "required": [DEFINITIONS_FIELD],
            "additionalProperties": false
        });

        Tool {
            name: Cow::Borrowed(READ_DEFINITIONS_TOOL_NAME),
            description: Some(Cow::Borrowed(READ_DEFINITIONS_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let input = ReadDefinitionsToolInput::new(params, &self.workspace_manager)?;

        let output = self.service.read_definitions(input)?;

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

    use crate::tools::{read_definitions::tool::ReadDefinitionsTool, types::KnowledgeGraphTool};

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_reads_single_definition_with_body() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &ReadDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // Test reading a single definition
        let result = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": ["bar"],
                        "file_path": "main/src/com/example/app/Foo.java"
                    }
                ]
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

        // Check the definitions
        let definitions = parsed["definitions"].as_array().unwrap();
        assert_eq!(definitions.len(), 1, "Expected exactly one definition");

        let definition = &definitions[0];
        assert_eq!(definition["name"].as_str().unwrap(), "bar");
        assert_eq!(
            definition["fqn"].as_str().unwrap(),
            "com.example.app.Foo.bar"
        );
        assert_eq!(definition["definition_type"].as_str().unwrap(), "Method");

        // Check that definition body is present
        assert!(
            definition["definition_body"].is_string(),
            "Expected definition body to be present"
        );
        let body = definition["definition_body"].as_str().unwrap();
        assert!(
            body.contains("return new Bar()"),
            "Expected method body content"
        );

        // Check that there's no error
        assert!(
            definition["definition_body_error"].is_null(),
            "Expected no definition body error"
        );

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_reads_multiple_definitions_efficiently() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &ReadDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // Test reading multiple definitions in one request
        let result = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": ["bar"],
                        "file_path": "main/src/com/example/app/Foo.java"
                    },
                    {
                        "names": ["main"],
                        "file_path": "main/src/com/example/app/Main.java"
                    }
                ]
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

        let definitions = parsed["definitions"].as_array().unwrap();
        assert_eq!(definitions.len(), 2, "Expected exactly two definitions");

        // Check first definition (bar method)
        let bar_def = definitions
            .iter()
            .find(|def| def["name"].as_str().unwrap() == "bar")
            .expect("Should find bar method definition");

        assert!(
            bar_def["definition_body"].is_string(),
            "Expected bar definition body"
        );
        let bar_body = bar_def["definition_body"].as_str().unwrap();
        assert!(
            bar_body.contains("return new Bar()"),
            "Expected bar method body content"
        );

        // Check second definition (main method)
        let main_def = definitions
            .iter()
            .find(|def| def["name"].as_str().unwrap() == "main")
            .expect("Should find main method definition");

        assert!(
            main_def["definition_body"].is_string(),
            "Expected main definition body"
        );
        let main_body = main_def["definition_body"].as_str().unwrap();
        assert!(
            main_body.contains("this.myParameter.bar()"),
            "Expected main method body content"
        );

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_reads_multiple_definitions_from_same_file() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &ReadDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // Test reading multiple definitions from the same file using names array
        let result = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": ["bar", "Foo"],
                        "file_path": "main/src/com/example/app/Foo.java"
                    }
                ]
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

        let definitions = parsed["definitions"].as_array().unwrap();
        // Should find at least one definition (the bar method)
        assert!(!definitions.is_empty(), "Expected at least one definition");

        // Check that we have the bar method
        let bar_def = definitions
            .iter()
            .find(|def| def["name"].as_str().unwrap() == "bar")
            .expect("Should find bar method definition");

        assert!(
            bar_def["definition_body"].is_string(),
            "Expected bar definition body"
        );
        let bar_body = bar_def["definition_body"].as_str().unwrap();
        assert!(
            bar_body.contains("return new Bar()"),
            "Expected bar method body content"
        );

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_handles_nonexistent_definitions_gracefully() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &ReadDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // Test with non-existent definition
        let result = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": ["nonExistentMethod"],
                        "file_path": "main/src/com/example/app/Foo.java"
                    }
                ]
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

        let definitions = parsed["definitions"].as_array().unwrap();
        assert_eq!(
            definitions.len(),
            0,
            "Expected no definitions for non-existent method"
        );

        let system_message = parsed["system_message"].as_str().unwrap();
        assert!(
            system_message.contains("No definitions were found"),
            "Expected appropriate system message"
        );

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_validates_input_parameters() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &ReadDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // Test missing definitions field
        let result_missing_definitions = tool.call(object(json!({})));
        assert!(
            result_missing_definitions.is_err(),
            "Should return error for missing definitions"
        );

        // Test empty definitions array
        let result_empty_definitions = tool.call(object(json!({
            "definitions": []
        })));
        assert!(
            result_empty_definitions.is_err(),
            "Should return error for empty definitions array"
        );

        // Test missing names field
        let result_missing_names = tool.call(object(json!({
            "definitions": [
                {
                    "file_path": "main/src/com/example/app/Foo.java"
                }
            ]
        })));
        assert!(
            result_missing_names.is_err(),
            "Should return error for missing names field"
        );

        // Test missing file_path field
        let result_missing_file_path = tool.call(object(json!({
            "definitions": [
                {
                    "names": ["bar"]
                }
            ]
        })));
        assert!(
            result_missing_file_path.is_err(),
            "Should return error for missing file_path field"
        );

        // Test empty names array
        let result_empty_names = tool.call(object(json!({
            "definitions": [
                {
                    "names": [],
                    "file_path": "main/src/com/example/app/Foo.java"
                }
            ]
        })));
        assert!(
            result_empty_names.is_err(),
            "Should return error for empty names array"
        );

        // Test empty name in names array
        let result_empty_name = tool.call(object(json!({
            "definitions": [
                {
                    "names": [""],
                    "file_path": "main/src/com/example/app/Foo.java"
                }
            ]
        })));
        assert!(
            result_empty_name.is_err(),
            "Should return error for empty name in names array"
        );

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_handles_file_not_in_workspace_error() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &ReadDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        // Test with a file that doesn't exist in the project
        let result = tool.call(object(json!({
            "definitions": [
                {
                    "names": ["someMethod"],
                    "file_path": "non/existent/path/NonExistent.java"
                }
            ]
        })));

        // This should return an error since the file is not in the workspace
        assert!(result.is_err(), "Expected error for invalid file path");

        setup.cleanup();
    }
}
