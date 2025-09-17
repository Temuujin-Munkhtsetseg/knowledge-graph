use std::{borrow::Cow, sync::Arc};

use rmcp::model::{CallToolResult, Content, JsonObject, Tool, object};
use serde_json::json;
use workspace_manager::WorkspaceManager;

use crate::tools::types::KnowledgeGraphTool;
use crate::tools::xml::XmlBuilder;

pub const LIST_PROJECTS_TOOL_NAME: &str = "list_projects";
pub const LIST_PROJECTS_TOOL_DESCRIPTION: &str = r#"Get a list of all projects indexed in the knowledge graph.

Useful for:
- You don't know the absolute filesystem path to the current project root directory.
- You want to know the indexed projects in the Knowledge Graph.
"#;

pub struct ListProjectsTool {
    workspace_manager: Arc<WorkspaceManager>,
}

impl ListProjectsTool {
    pub fn new(workspace_manager: Arc<WorkspaceManager>) -> Self {
        Self { workspace_manager }
    }
}

#[async_trait::async_trait]
impl KnowledgeGraphTool for ListProjectsTool {
    fn name(&self) -> &str {
        LIST_PROJECTS_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let input_schema = json!({
            "type": "object",
            "properties": {},
            "required": []
        });

        Tool {
            name: Cow::Borrowed(LIST_PROJECTS_TOOL_NAME),
            description: Some(Cow::Borrowed(LIST_PROJECTS_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    async fn call(&self, _params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let projects = self.workspace_manager.list_all_projects();

        let mut builder = XmlBuilder::new();
        builder.start_element("ToolResponse").unwrap();
        builder.start_element("projects").unwrap();

        for project in projects {
            builder
                .write_element("project_path", &project.project_path)
                .unwrap();
        }

        builder.end_element("projects").unwrap();
        builder.end_element("ToolResponse").unwrap();

        let xml_output = builder.finish().unwrap();

        Ok(CallToolResult::success(vec![Content::text(xml_output)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_list_projects_tool_functionality() {
        let (workspace_manager, _project_path) = create_test_workspace_manager();

        let tool = ListProjectsTool::new(workspace_manager.clone());

        let empty_params = JsonObject::new();
        let result = futures::executor::block_on(tool.call(empty_params)).unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let content = result.content.as_ref().unwrap();
        assert_eq!(content.len(), 1);

        let content = &content[0];
        let xml_data = content.as_text().unwrap().text.clone();

        assert!(xml_data.contains("<ToolResponse>"));
        assert!(xml_data.contains("<projects>"));
        assert!(xml_data.contains("<project_path>"));
        assert!(xml_data.contains("test_project"));
    }
}
