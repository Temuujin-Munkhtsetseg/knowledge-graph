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
use crate::tools::{
    get_references::service::GetReferencesService, types::KnowledgeGraphTool, xml::ToXml,
};

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

#[async_trait::async_trait]
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
                    "description": "Absolute file path to the file that contains the symbol usage. Example: /abs/path/to/src/main/java/com/example/User.java"
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

    async fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let input = GetReferencesToolInput::new(params, &self.workspace_manager)?;

        let output = self.service.get_references(input).await?;

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
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 1,
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
        assert!(
            xml_str.contains("<definition>"),
            "Expected at least one definition"
        );

        // Check that next_page is not present (should be empty/null)
        assert!(
            !xml_str.contains("<next-page>"),
            "Expected no next-page element"
        );

        // Check for main method definition
        assert!(
            xml_str.contains("<fqn>com.example.app.Main.main</fqn>"),
            "Expected main method definition"
        );
        assert!(
            xml_str.contains("<name>main</name>"),
            "Expected main method name"
        );
        assert!(
            xml_str.contains(&format!(
                "<location>{}/main/src/com/example/app/Main.java:L15-44</location>",
                project.project_path
            )),
            "Expected location element"
        );
        assert!(
            xml_str.contains("<definition-type>Method</definition-type>"),
            "Expected definition-type element"
        );

        // Check the reference within this definition
        assert!(
            xml_str.contains("<references>"),
            "Expected references element"
        );
        assert!(
            xml_str.contains("<reference>"),
            "Expected at least one reference"
        );
        assert!(
            xml_str.contains("<reference-type>CALLS</reference-type>"),
            "Expected CALLS reference type"
        );
        assert!(
            xml_str.contains(&format!(
                "<location>{}/main/src/com/example/app/Main.java:L17-17</location>",
                project.project_path
            )),
            "Expected reference location"
        );
        assert!(
            xml_str.contains("<context>"),
            "Expected context element in CDATA"
        );
        assert!(xml_str.contains("@Traceable"), "Expected context content");
        assert!(
            xml_str.contains("public void main() {"),
            "Expected main method signature in context"
        );
        assert!(
            xml_str.contains("this.myParameter.bar()"),
            "Expected method call in context"
        );
        assert!(xml_str.contains("}"), "Expected closing brace in context");
        assert!(
            xml_str.contains("</context>"),
            "Expected closing context element"
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
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Bar.java",
                "page": 1,
            })))
            .await
            .unwrap();

        let content = result.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content[0];
        let xml_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure for Bar class references
        assert!(
            xml_str.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str.contains("<definitions>"),
            "Expected definitions element"
        );
        assert!(
            !xml_str.contains("<next-page>"),
            "Expected no next-page element"
        );

        // Find the bar method definition
        assert!(
            xml_str.contains("<fqn>com.example.app.Foo.bar</fqn>"),
            "Expected Foo.bar method definition"
        );
        assert!(
            xml_str.contains("<name>bar</name>"),
            "Expected bar method name"
        );
        assert!(
            xml_str.contains(&format!(
                "<location>{}/main/src/com/example/app/Foo.java:L6-8</location>",
                project.project_path
            )),
            "Expected bar method location"
        );
        assert!(
            xml_str.contains("<definition-type>Method</definition-type>"),
            "Expected definition-type element"
        );

        // Check the reference within this definition (constructor call)
        assert!(
            xml_str.contains("<references>"),
            "Expected references element"
        );
        assert!(
            xml_str.contains("<reference-type>CALLS</reference-type>"),
            "Expected CALLS reference type"
        );
        assert!(
            xml_str.contains(&format!(
                "<location>{}/main/src/com/example/app/Foo.java:L7-7</location>",
                project.project_path
            )),
            "Expected reference location"
        );
        assert!(
            xml_str.contains("<context>"),
            "Expected context element in CDATA"
        );
        assert!(
            xml_str.contains("public Bar bar() {"),
            "Expected method signature in context"
        );
        assert!(
            xml_str.contains("return new Bar()"),
            "Expected constructor call in context"
        );
        assert!(xml_str.contains("}"), "Expected closing brace in context");

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

        let project = &setup.workspace_manager.clone().list_all_projects()[0];

        // First, get all results to see total count
        let result_all = tool
            .call(object(json!({
                "definition_name": "Foo",
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 1,
            })))
            .await
            .unwrap();

        let content_all = result_all.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content_all[0];
        let xml_str_all = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure for pagination test (all results)
        assert!(
            xml_str_all.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str_all.contains("<definitions>"),
            "Expected definitions element"
        );

        assert!(
            xml_str_all.contains("<next-page>2</next-page>"),
            "Expected next-page element with value 2"
        );

        // Test pagination by limiting to 1 result
        let result_limited = tool
            .call(object(json!({
                "definition_name": "Foo",
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 2,
            })))
            .await
            .unwrap();

        let content_limited = result_limited.content.expect("Expected content in result");
        let rmcp::model::Annotated { raw, .. } = &content_limited[0];
        let xml_str_limited = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure for pagination test (limited results)
        assert!(
            xml_str_limited.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str_limited.contains("<definitions>"),
            "Expected definitions element"
        );
        assert!(
            !xml_str_limited.contains("<next-page>"),
            "Expected no next-page element"
        );

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
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 1,
            })))
            .await
            .unwrap();

        let content = result.content.expect("Expected content in result");
        assert!(!content.is_empty(), "Expected non-empty content");

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
        assert!(
            !xml_str.contains("<definition>"),
            "Expected no definitions for non-existent method"
        );
        assert!(
            !xml_str.contains("<next-page>"),
            "Expected no next-page element"
        );

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
        let result = tool
            .call(object(json!({
                "definition_name": "bar",
                "absolute_file_path": "/non/existent/path/NonExistent.java",
                "page": 1,
            })))
            .await;

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

        let project = &setup.workspace_manager.clone().list_all_projects()[0];

        let result_large_page = tool
            .call(object(json!({
                "definition_name": "bar",
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 999999,
            })))
            .await;
        assert!(
            result_large_page.is_ok(),
            "Should handle oversized limit gracefully"
        );

        let result_zero_page = tool
            .call(object(json!({
                "definition_name": "bar",
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 0,
            })))
            .await;
        assert!(
            result_zero_page.is_ok(),
            "Should handle zero limit gracefully"
        );

        let result_negative_page = tool
            .call(object(json!({
                "definition_name": "bar",
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": -10,
            })))
            .await;
        assert!(
            result_negative_page.is_ok(),
            "Should handle negative offset gracefully"
        );

        let result_missing_definition = tool
            .call(object(json!({
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 1,
            })))
            .await;
        assert!(
            result_missing_definition.is_err(),
            "Should return error for missing definition_name"
        );

        let result_missing_file_path = tool
            .call(object(json!({
                "definition_name": "bar",
                "page": 1,
            })))
            .await;
        assert!(
            result_missing_file_path.is_err(),
            "Should return error for missing absolute_file_path"
        );

        let result_empty_definition = tool
            .call(object(json!({
                "definition_name": "",
                "absolute_file_path": project.project_path.clone() + "/main/src/com/example/app/Foo.java",
                "page": 1,
            })))
            .await;
        assert!(
            result_empty_definition.is_err(),
            "Should return error for empty definition_name"
        );

        setup.cleanup();
    }
}
