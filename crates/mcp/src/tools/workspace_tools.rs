use rmcp::model::{CallToolResult, Content, JsonObject, Tool};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::sync::Arc;

use workspace_manager::WorkspaceManager;

use crate::tools::types::KnowledgeGraphTool;

pub struct ListProjectsTool {
    workspace_manager: Arc<WorkspaceManager>,
}

impl ListProjectsTool {
    pub fn new(workspace_manager: Arc<WorkspaceManager>) -> Self {
        Self { workspace_manager }
    }
}

impl KnowledgeGraphTool for ListProjectsTool {
    fn name(&self) -> &str {
        "list_projects"
    }

    fn to_mcp_tool(&self) -> Tool {
        let mut input_schema = JsonObject::new();
        input_schema.insert("type".to_string(), Value::String("object".to_string()));
        input_schema.insert("properties".to_string(), Value::Object(JsonObject::new()));
        input_schema.insert("required".to_string(), Value::Array(vec![]));

        Tool {
            name: Cow::Borrowed("list_projects"),
            description: Some(Cow::Borrowed(
                "Get a list of all projects in the knowledge graph.",
            )),
            input_schema: Arc::new(input_schema),
            annotations: None,
        }
    }

    fn call(&self, _params: JsonObject) -> Result<CallToolResult, rmcp::Error> {
        let projects = self.workspace_manager.list_all_projects();

        let project_data: Vec<Value> = projects
            .into_iter()
            .map(|project| {
                json!({
                    "project_path": project.project_path,
                })
            })
            .collect();

        let result = json!({
            "projects": project_data,
        });

        Ok(CallToolResult::success(vec![
            Content::json(result).map_err(|e| {
                rmcp::Error::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    format!("Failed to serialize result: {e}"),
                    None,
                )
            })?,
        ]))
    }
}

pub fn get_list_projects_tool(workspace_manager: Arc<WorkspaceManager>) -> ListProjectsTool {
    ListProjectsTool::new(workspace_manager)
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

        // Test tool creation
        let tool = get_list_projects_tool(workspace_manager.clone());
        assert_eq!(tool.name(), "list_projects");

        // Test MCP tool conversion
        let mcp_tool = tool.to_mcp_tool();
        assert_eq!(mcp_tool.name, "list_projects");
        assert_eq!(
            mcp_tool.description.as_ref().unwrap(),
            "Get a list of all projects in the knowledge graph."
        );

        // Test tool execution
        let empty_params = JsonObject::new();
        let result = tool.call(empty_params).unwrap();

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(result.content.len(), 1);

        let content = &result.content[0];
        let json_data: Value = serde_json::from_str(&content.as_text().unwrap().text).unwrap();

        assert!(json_data["projects"].is_array());
        let projects = json_data["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 1);

        let project = &projects[0];
        assert!(project["project_path"].is_string());
        assert!(
            project["project_path"]
                .as_str()
                .unwrap()
                .contains("test_project")
        );
    }
}
