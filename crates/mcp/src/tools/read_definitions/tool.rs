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
use crate::tools::{
    read_definitions::service::ReadDefinitionsService, types::KnowledgeGraphTool, xml::ToXml,
};

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

#[async_trait::async_trait]
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

    async fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let input = ReadDefinitionsToolInput::new(params, &self.workspace_manager)?;

        let output = self.service.read_definitions(input).await?;

        let xml_output = output.to_xml_without_cdata().map_err(|e| {
            rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Failed to convert output to XML: {}", e),
                None,
            )
        })?;

        Ok(CallToolResult::success(vec![Content::text(xml_output)]))
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
            .await
            .unwrap();

        let content = result.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content[0];
        let xml_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure
        assert!(
            xml_str.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str.contains("</ToolResponse>"),
            "Expected closing ToolResponse element"
        );
        assert!(
            xml_str.contains("<definitions>"),
            "Expected definitions element"
        );
        assert!(
            xml_str.contains("</definitions>"),
            "Expected closing definitions element"
        );
        assert!(
            xml_str.contains("<definition>"),
            "Expected definition element"
        );
        assert!(
            xml_str.contains("</definition>"),
            "Expected closing definition element"
        );
        assert!(
            xml_str.contains("<name>bar</name>"),
            "Expected name element with 'bar'"
        );
        assert!(
            xml_str.contains("<fqn>com.example.app.Foo.bar</fqn>"),
            "Expected fqn element"
        );
        assert!(
            xml_str.contains("<definition-type>Method</definition-type>"),
            "Expected definition-type element"
        );

        // Check that definition body is present in CDATA
        assert!(
            xml_str.contains("<definition-body>"),
            "Expected definition-body element"
        );
        assert!(
            xml_str.contains("</definition-body>"),
            "Expected closing definition-body element"
        );
        assert!(
            xml_str.contains("return new Bar()"),
            "Expected method body content in CDATA"
        );

        // Check that system message is present
        assert!(
            xml_str.contains("<system-message>"),
            "Expected system-message element"
        );
        assert!(
            xml_str.contains("</system-message>"),
            "Expected closing system-message element"
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
            .await
            .unwrap();

        let content = result.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content[0];
        let xml_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure for multiple definitions
        assert!(
            xml_str.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str.contains("<definitions>"),
            "Expected definitions element"
        );

        // Check that we have two definition elements
        let definition_count = xml_str.matches("<definition>").count();
        assert_eq!(definition_count, 2, "Expected exactly two definitions");

        // Check first definition (bar method)
        assert!(
            xml_str.contains("<name>bar</name>"),
            "Expected bar method definition"
        );
        assert!(
            xml_str.contains("return new Bar()"),
            "Expected bar method body content"
        );

        // Check second definition (baz method)
        assert!(
            xml_str.contains("<name>main</name>"),
            "Expected main method definition"
        );
        assert!(
            xml_str.contains("this.myParameter.bar()"),
            "Expected method body content referencing bar()"
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
            .await
            .unwrap();

        let content = result.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content[0];
        let xml_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure
        assert!(
            xml_str.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str.contains("<definitions>"),
            "Expected definitions element"
        );

        // Should find at least one definition element
        assert!(
            xml_str.contains("<definition>"),
            "Expected at least one definition"
        );

        // Check that we have the bar method
        assert!(
            xml_str.contains("<name>bar</name>"),
            "Expected bar method definition"
        );
        assert!(
            xml_str.contains("return new Bar()"),
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
            .await
            .unwrap();

        let content = result.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content[0];
        let xml_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure for empty result
        assert!(
            xml_str.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str.contains("<definitions>"),
            "Expected definitions element"
        );
        assert!(
            xml_str.contains("</definitions>"),
            "Expected closing definitions element"
        );

        // Should have no definition elements
        assert!(
            !xml_str.contains("<definition>"),
            "Expected no definitions for non-existent method"
        );

        // Check system message
        assert!(
            xml_str.contains("<system-message>"),
            "Expected system-message element"
        );
        assert!(
            xml_str.contains("No definitions were found"),
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
        let result_missing_definitions = tool.call(object(json!({}))).await;
        assert!(
            result_missing_definitions.is_err(),
            "Should return error for missing definitions"
        );

        // Test empty definitions array
        let result_empty_definitions = tool
            .call(object(json!({
                "definitions": []
            })))
            .await;
        assert!(
            result_empty_definitions.is_err(),
            "Should return error for empty definitions array"
        );

        // Test missing names field
        let result_missing_names = tool
            .call(object(json!({
                "definitions": [
                    {
                        "file_path": "main/src/com/example/app/Foo.java"
                    }
                ]
            })))
            .await;
        assert!(
            result_missing_names.is_err(),
            "Should return error for missing names field"
        );

        // Test missing file_path field
        let result_missing_file_path = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": ["bar"]
                    }
                ]
            })))
            .await;
        assert!(
            result_missing_file_path.is_err(),
            "Should return error for missing file_path field"
        );

        // Test empty names array
        let result_empty_names = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": [],
                        "file_path": "main/src/com/example/app/Foo.java"
                    }
                ]
            })))
            .await;
        assert!(
            result_empty_names.is_err(),
            "Should return error for empty names array"
        );

        // Test empty name in names array
        let result_empty_name = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": [""],
                        "file_path": "main/src/com/example/app/Foo.java"
                    }
                ]
            })))
            .await;
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
        let result = tool
            .call(object(json!({
                "definitions": [
                    {
                        "names": ["someMethod"],
                        "file_path": "non/existent/path/NonExistent.java"
                    }
                ]
            })))
            .await;

        // This should return an error since the file is not in the workspace
        assert!(result.is_err(), "Expected error for invalid file path");

        setup.cleanup();
    }
}
