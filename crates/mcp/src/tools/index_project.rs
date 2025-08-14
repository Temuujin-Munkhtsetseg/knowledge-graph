use std::borrow::Cow;
use std::sync::Arc;
use std::thread;

use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use indexer::execution::{config::IndexingConfigBuilder, executor::IndexingExecutor};
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, Tool};
use serde_json::Value;
use tokio::runtime::Builder;
use workspace_manager::WorkspaceManager;

use crate::tools::types::KnowledgeGraphTool;

pub const INDEX_PROJECT_TOOL_NAME: &str = "index_project";
const INDEX_PROJECT_TOOL_DESCRIPTION: &str = "Rebuilds the Knowledge Graph index for a project to reflect recent changes. Use this tool when: \
- You have made substantial modifications to project files, structure, or content \
- The Knowledge Graph seems outdated or missing recent changes \
- Search results are not reflecting recent updates to the project \
- After bulk operations like file imports, deletions, or major refactoring";

pub struct IndexProjectTool {
    database: Arc<KuzuDatabase>,
    workspace_manager: Arc<WorkspaceManager>,
    event_bus: Arc<EventBus>,
}

impl IndexProjectTool {
    pub fn new(
        database: Arc<KuzuDatabase>,
        workspace_manager: Arc<WorkspaceManager>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            database,
            workspace_manager,
            event_bus,
        }
    }
}

impl KnowledgeGraphTool for IndexProjectTool {
    fn name(&self) -> &str {
        INDEX_PROJECT_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let mut properties = JsonObject::new();

        let mut project_property = JsonObject::new();
        project_property.insert("type".to_string(), Value::String("string".to_string()));
        project_property.insert(
            "description".to_string(),
            Value::String(
                "The absolute path to the current project directory to re-index synchronously."
                    .to_string(),
            ),
        );
        properties.insert(
            "project_absolute_path".to_string(),
            Value::Object(project_property),
        );

        let mut input_schema = JsonObject::new();
        input_schema.insert("type".to_string(), Value::String("object".to_string()));
        input_schema.insert("properties".to_string(), Value::Object(properties));
        input_schema.insert(
            "required".to_string(),
            Value::Array(vec![Value::String("project_absolute_path".to_string())]),
        );

        Tool {
            name: Cow::Borrowed(INDEX_PROJECT_TOOL_NAME),
            description: Some(Cow::Borrowed(INDEX_PROJECT_TOOL_DESCRIPTION)),
            input_schema: Arc::new(input_schema),
            output_schema: None,
            annotations: None,
        }
    }

    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let project_absolute_path = params
            .get("project_absolute_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Missing project_absolute_path".to_string(),
                    None,
                )
            })?;

        // Resolve workspace for the project
        let project_info = self
            .workspace_manager
            .get_project_for_path(project_absolute_path)
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Project not found in workspace manager".to_string(),
                    None,
                )
            })?;

        let database = Arc::clone(&self.database);
        let workspace_manager = Arc::clone(&self.workspace_manager);
        let event_bus = Arc::clone(&self.event_bus);
        let workspace_folder_path = project_info.workspace_folder_path.clone();
        let project_path = project_info.project_path.clone();

        let handle = thread::spawn(move || {
            let runtime = Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to build tokio runtime: {e}"),
                        None,
                    )
                })?;

            runtime.block_on(async move {
                let threads = num_cpus::get();
                let config = IndexingConfigBuilder::build(threads);
                let mut executor =
                    IndexingExecutor::new(database, workspace_manager, event_bus, config);

                executor
                    .execute_project_indexing(&workspace_folder_path, &project_path, None)
                    .await
                    .map_err(|e| {
                        rmcp::ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            format!("Re-index failed: {e}"),
                            None,
                        )
                    })
            })
        });

        let project_stats = handle.join().map_err(|_| {
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Indexing thread panicked".to_string(),
                None,
            )
        })??;

        let mut result = JsonObject::new();
        result.insert("status".to_string(), Value::String("ok".to_string()));
        result.insert(
            "stats".to_string(),
            serde_json::to_value(project_stats).map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to serialize stats: {e}"),
                    None,
                )
            })?,
        );

        Ok(CallToolResult::success(vec![
            Content::json(result).unwrap(),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::TempDir;
    use testing::repository::TestRepository;

    fn create_workspace_with_project() -> (TempDir, TempDir, Arc<WorkspaceManager>, String) {
        let temp_workspace_dir = TempDir::new().unwrap();
        let workspace_path = temp_workspace_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let project_path = workspace_path.join("test_project");
        TestRepository::new(&project_path, Some("test-repo"));

        let temp_data_dir = TempDir::new().unwrap();
        let manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );

        manager.register_workspace_folder(&workspace_path).unwrap();

        let projects = manager.list_all_projects();
        let project_path = projects[0].project_path.clone();

        (temp_workspace_dir, temp_data_dir, manager, project_path)
    }

    #[test]
    fn test_index_project_returns_stats() {
        let (_workspace_dir, _data_dir, workspace_manager, project_path) =
            create_workspace_with_project();
        let database = Arc::new(KuzuDatabase::new());
        let event_bus = Arc::new(EventBus::new());

        let tool = IndexProjectTool::new(
            Arc::clone(&database),
            Arc::clone(&workspace_manager),
            Arc::clone(&event_bus),
        );

        let mut params = JsonObject::new();
        params.insert(
            "project_absolute_path".to_string(),
            Value::String(project_path.clone()),
        );

        let result = tool.call(params).expect("tool call should succeed");
        let text = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();

        let obj: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(obj.get("status").and_then(|v| v.as_str()).unwrap(), "ok");

        let stats = obj
            .get("stats")
            .and_then(|v| v.as_object())
            .expect("stats should be an object");

        let returned_project_path = stats
            .get("project_path")
            .and_then(|v| v.as_str())
            .expect("project_path should exist");
        assert_eq!(returned_project_path, project_path);

        let total_files = stats
            .get("total_files")
            .and_then(|v| v.as_u64())
            .expect("total_files should exist");
        assert!(total_files >= 1);
    }
}
