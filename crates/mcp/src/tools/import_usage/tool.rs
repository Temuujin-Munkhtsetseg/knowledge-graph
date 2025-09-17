use std::borrow::Cow;
use std::sync::Arc;

use database::querying::QueryingService;
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, Tool, object};
use serde_json::json;
use workspace_manager::WorkspaceManager;

use crate::tools::import_usage::constants::{
    IMPORT_USAGE_TOOL_DESCRIPTION, IMPORT_USAGE_TOOL_NAME,
};
use crate::tools::import_usage::input::ImportUsageInput;
use crate::tools::import_usage::service::ImportUsageService;
use crate::tools::types::KnowledgeGraphTool;
use crate::tools::xml::ToXml;

pub struct ImportUsageTool {
    workspace_manager: Arc<WorkspaceManager>,
    service: ImportUsageService,
}

impl ImportUsageTool {
    pub fn new(
        querying_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            workspace_manager: Arc::clone(&workspace_manager),
            service: ImportUsageService::new(querying_service),
        }
    }
}

#[async_trait::async_trait]
impl KnowledgeGraphTool for ImportUsageTool {
    fn name(&self) -> &str {
        IMPORT_USAGE_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let input_schema = json!({
            "type": "object",
            "properties": {
                "project_absolute_path": {
                    "type": "string",
                    "description": "Absolute path to the project root directory."
                },
                "packages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "import_path": { "type": "string" },
                            "name": { "type": "string" },
                            "alias": { "type": "string" },
                        },
                        "required": ["import_path"],
                        "additionalProperties": false
                    },
                    "minItems": 1
                },
                "page": { "type": "integer", "minimum": 1, "default": 1 },
                "page_size": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
            },
            "required": ["project_absolute_path", "packages"],
            "additionalProperties": false
        });

        Tool {
            name: Cow::Borrowed(IMPORT_USAGE_TOOL_NAME),
            description: Some(Cow::Borrowed(IMPORT_USAGE_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    async fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let input = ImportUsageInput::new(params, &self.workspace_manager)?;
        let output = self.service.analyze(input).await?;
        let xml = output
            .to_xml_without_cdata()
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(xml)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::index_project::IndexProjectTool;
    use crate::tools::types::KnowledgeGraphTool;
    use database::kuzu::database::KuzuDatabase;
    use database::querying::DatabaseQueryingService;
    use event_bus::EventBus;
    use rmcp::model::object;
    use serde_json::json;
    use tempfile::TempDir;
    use testing::repository::TestRepository;

    fn setup_java_workspace() -> (TempDir, TempDir, Arc<WorkspaceManager>, String) {
        let temp_workspace_dir = TempDir::new().unwrap();
        let workspace_path = temp_workspace_dir.path().join("java_workspace_e2e");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let project_path = workspace_path.join("java_project");
        TestRepository::new(&project_path, Some("java-user-service"));

        let temp_data_dir = TempDir::new().unwrap();
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );
        let _folder = workspace_manager
            .register_workspace_folder(&workspace_path)
            .unwrap();
        let projects = workspace_manager.list_all_projects();
        assert!(
            !projects.is_empty(),
            "Workspace should discover at least one project"
        );
        let registered_project_path = projects[0].project_path.clone();
        (
            temp_workspace_dir,
            temp_data_dir,
            workspace_manager,
            registered_project_path,
        )
    }

    async fn index_project(workspace_manager: &Arc<WorkspaceManager>, project_path: &str) {
        let database = Arc::new(KuzuDatabase::new());
        let event_bus = Arc::new(EventBus::new());
        let index_tool = IndexProjectTool::new(
            Arc::clone(&database),
            Arc::clone(workspace_manager),
            Arc::clone(&event_bus),
        );
        let mut index_params = JsonObject::new();
        index_params.insert(
            "project_absolute_path".to_string(),
            serde_json::Value::String(project_path.to_string()),
        );
        let index_result = index_tool.call(index_params).await;
        assert!(index_result.is_ok(), "Indexing should succeed");
    }

    fn make_tool(
        database: Arc<KuzuDatabase>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> impl KnowledgeGraphTool {
        ImportUsageTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::clone(&workspace_manager),
        )
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_import_usage_java_spring_imports_and_refs() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_java_workspace();
        index_project(&workspace_manager, &project_path).await;

        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // Analyze a known import path present in the controller
        // e.g., org.springframework.web.bind.annotation
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [
                    { "import_path": "org.springframework.web.bind.annotation" }
                ]
            })))
            .await
            .unwrap();

        let xml = match &result.content.unwrap()[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };

        // Basic assertions: imports and definitions containers present
        assert!(xml.contains("<imports>"));
        assert!(xml.contains("<usages>"));
        // Ensure we have at least one import line
        assert!(xml.contains("import org.springframework.web.bind.annotation"));
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_import_usage_java_logging_import() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_java_workspace();
        index_project(&workspace_manager, &project_path).await;

        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // Validate log4j import appears in imports block
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [
                    { "import_path": "org.apache.logging.log4j" }
                ]
            })))
            .await
            .unwrap();

        let xml = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(xml.contains("import org.apache.logging.log4j"));
        assert!(xml.contains("UserController.java"));
        assert!(xml.contains("getUserById"));
        assert!(xml.contains("getAllUsers"));
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_import_usage_case_insensitive_import_path() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_java_workspace();
        index_project(&workspace_manager, &project_path).await;

        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // Uppercased import path should still match (query lowers both sides)
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [
                    { "import_path": "ORG.APACHE.LOGGING.LOG4J" }
                ]
            })))
            .await
            .unwrap();

        let xml = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(xml.contains("import org.apache.logging.log4j"));
    }

    fn setup_ts_workspace() -> (TempDir, TempDir, Arc<WorkspaceManager>, String) {
        let temp_workspace_dir = TempDir::new().unwrap();
        let workspace_path = temp_workspace_dir.path().join("ts_workspace_dep_e2e");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let project_path = workspace_path.join("ts_project");
        TestRepository::new(&project_path, Some("typescript/test-repo"));

        let temp_data_dir = TempDir::new().unwrap();
        let workspace_manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );
        let _folder = workspace_manager
            .register_workspace_folder(&workspace_path)
            .unwrap();
        let projects = workspace_manager.list_all_projects();
        assert!(
            !projects.is_empty(),
            "Workspace should discover at least one project"
        );
        let registered_project_path = projects[0].project_path.clone();
        (
            temp_workspace_dir,
            temp_data_dir,
            workspace_manager,
            registered_project_path,
        )
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_import_usage_aliases() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_ts_workspace();
        index_project(&workspace_manager, &project_path).await;

        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // no alias
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [
                    { "import_path": "crypto" }
                ]
            })))
            .await
            .unwrap();

        let xml = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(xml.contains("import "));
        assert!(xml.contains("crypto"));
        assert!(xml.contains(" L"));
        assert!(xml.contains("base_model.ts"));

        // with alias
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [
                    { "import_path": "crypto", "alias": "myRandomUUID" }
                ]
            })))
            .await
            .unwrap();

        let xml = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(xml.contains("myRandomUUID"));
        assert!(xml.contains("base_model.ts"));
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_import_usage_typescript_relative_import() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_ts_workspace();
        index_project(&workspace_manager, &project_path).await;

        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // page defaults to 1, page_size=50; verify the XML contains imports but not more than page window if many
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [
                    { "import_path": "./app/models/user_model" }
                ],
                "page": 1,
                "page_size": 50
            })))
            .await
            .unwrap();

        let xml = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(xml.contains("import "));
        assert!(xml.contains("./app/models/user_model"));
        assert!(xml.contains(" L"));
        assert!(xml.contains("main.ts"));
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_import_usage_pagination_limits_usages() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_java_workspace();
        index_project(&workspace_manager, &project_path).await;

        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // First page
        let page1 = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [ { "import_path": "org.apache.logging.log4j" } ],
                "page": 1,
                "page_size": 1
            })))
            .await
            .unwrap();
        let xml1 = page1.content.as_ref().unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        // extract the file path from the xml
        let xml1_file_path = xml1
            .split("<path>")
            .nth(1)
            .and_then(|s| s.split("</path>").next())
            .unwrap_or("")
            .trim()
            .to_string();
        assert!(!xml1_file_path.is_empty());
        assert!(xml1.contains(xml1_file_path.as_str()));
        assert!(xml1.contains("<next-page>2</next-page>") || xml1.contains("<next-page>"));
        // Second page
        let page2 = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "packages": [ { "import_path": "org.apache.logging.log4j" } ],
                "page": 2,
                "page_size": 1
            })))
            .await
            .unwrap();
        let xml2 = page2.content.as_ref().unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(
            !xml2.contains(xml1_file_path.as_str()),
            "second page should not contain the same file"
        );
    }
}
