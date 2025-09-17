use std::borrow::Cow;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use database::querying::QueryingService;
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, Tool, object};
use serde_json::json;
use workspace_manager::WorkspaceManager;

use crate::tools::types::KnowledgeGraphTool;
use crate::tools::utils::get_database_path;

use super::constants::{
    DEFAULT_DEPTH, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MIN_PAGE,
    REPO_MAP_TOOL_DESCRIPTION, REPO_MAP_TOOL_NAME,
};
use super::input::RepoMapInput;
use super::output::{RepoMapItem, build_repo_map_xml};
use super::repository::collect_paths_ignore;
use super::service::RepoMapService;
use crate::tools::file_reader_utils::read_file_chunks;
use tokio::time::{Duration, timeout};

pub struct RepoMapTool {
    pub query_service: Arc<dyn QueryingService>,
    pub workspace_manager: Arc<WorkspaceManager>,
}

impl RepoMapTool {
    pub fn new(
        query_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            query_service,
            workspace_manager,
        }
    }

    fn build_input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "project_absolute_path": { "type": "string", "description": "Absolute path to the project root directory." },
                "relative_paths": { "type": "array", "description": "Project-relative paths; each item may be a file or a directory under the project root. Directories are expanded recursively to files.", "items": { "type": "string" }, "minItems": 1 },
                "depth": { "type": "integer", "description": "Desired nesting depth for showing definition nodes for files under directories (advisory, included in output). 1 = top-level only. Maximum 3.", "default": DEFAULT_DEPTH, "minimum": 1, "maximum": 3 },
                "show_directories": { "type": "boolean", "description": "Whether to include the directories list.", "default": true },
                "show_definitions": { "type": "boolean", "description": "Whether to include files and their definitions.", "default": true },
                "page": { "type": "integer", "description": "Page number starting from 1.", "default": DEFAULT_PAGE, "minimum": MIN_PAGE },
                "page_size": { "type": "integer", "description": "Number of definitions per page (global across all files).", "default": DEFAULT_PAGE_SIZE, "minimum": 1, "maximum": MAX_PAGE_SIZE }
            },
            "required": ["project_absolute_path", "relative_paths"],
            "additionalProperties": false
        })
    }
}

#[async_trait::async_trait]
impl KnowledgeGraphTool for RepoMapTool {
    fn name(&self) -> &str {
        REPO_MAP_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        Tool {
            name: Cow::Borrowed(REPO_MAP_TOOL_NAME),
            description: Some(Cow::Borrowed(REPO_MAP_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(self.build_input_schema())),
            output_schema: None,
            annotations: None,
        }
    }

    async fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let service = RepoMapService {
            query_service: &*self.query_service,
            workspace_manager: &self.workspace_manager,
        };
        let input = RepoMapInput::try_from(params)?;

        let database_path =
            get_database_path(&self.workspace_manager, &input.project_absolute_path)?;
        let project_root = Path::new(&input.project_absolute_path)
            .canonicalize()
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INVALID_REQUEST, e.to_string(), None))?;

        let (expanded_files, collected_directories_rel) =
            collect_paths_ignore(&project_root, &input.relative_paths, input.depth)?;

        if expanded_files.is_empty() {
            let xml = build_repo_map_xml(Vec::new(), collected_directories_rel.clone(), input.show_directories, input.show_definitions, None, input.depth, "No files found within the specified project. Ensure paths are relative to the project root and exist.".to_string())
                .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
            return Ok(CallToolResult::success(vec![Content::text(xml)]));
        }

        let mut relative_files: Vec<String> = Vec::with_capacity(expanded_files.len());
        for abs in &expanded_files {
            if let Some(rel) = RepoMapService::to_relative(&project_root, abs) {
                relative_files.push(rel);
            }
        }

        let rows = service.query_definitions(
            database_path.clone(),
            relative_files,
            input.page,
            input.page_size.min(MAX_PAGE_SIZE),
        )?;
        if rows.is_empty() {
            let msg = format!(
                "No indexed definitions found for the requested paths under project {}. depth= {}.",
                project_root.display(),
                input.depth
            );
            let xml = build_repo_map_xml(
                Vec::new(),
                collected_directories_rel.clone(),
                input.show_directories,
                input.show_definitions,
                None,
                input.depth,
                msg,
            )
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
            return Ok(CallToolResult::success(vec![Content::text(xml)]));
        }

        // Prepare snippet reads
        let mut chunks: Vec<(String, usize, usize)> = Vec::with_capacity(rows.len());
        let mut abs_paths: Vec<String> = Vec::with_capacity(rows.len());
        for row in &rows {
            let file_abs = {
                let rel_path = Path::new(&row.file_path);
                let candidate = if rel_path.is_absolute() {
                    rel_path.to_path_buf()
                } else {
                    project_root.join(rel_path)
                };
                candidate.to_string_lossy().to_string()
            };
            let start_1 = row.start_line + 1;
            let end_1 = row.end_line + 1;
            let snippet_start = start_1;
            let snippet_end = std::cmp::min(start_1 + 2, end_1);
            chunks.push((file_abs.clone(), snippet_start, snippet_end));
            abs_paths.push(file_abs);
        }

        let file_contents: Vec<std::io::Result<String>> = if chunks.is_empty() {
            Vec::new()
        } else {
            match timeout(Duration::from_secs(10), read_file_chunks(chunks)).await {
                Ok(Ok(results)) => results,
                Ok(Err(e)) => {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        e.to_string(),
                        None,
                    ));
                }
                Err(_) => {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        "File reading operation timed out.".to_string(),
                        None,
                    ));
                }
            }
        };

        let mut items: Vec<RepoMapItem> = Vec::with_capacity(rows.len());
        for (idx, row) in rows.into_iter().enumerate() {
            let start_line_1 = row.start_line + 1;
            let end_line_1 = row.end_line + 1;
            let snippet = match file_contents.get(idx) {
                Some(Ok(s)) => Some(s.trim_end_matches('\n').to_string()),
                _ => None,
            };
            let file_rel_norm = if Path::new(&row.file_path).is_absolute() {
                RepoMapService::to_relative(&project_root, &row.file_path).unwrap_or(row.file_path)
            } else {
                row.file_path
            };
            items.push(RepoMapItem {
                file_rel: file_rel_norm,
                fqn: row.fqn,
                def_type: row.definition_type,
                start_line_1,
                end_line_1,
                snippet,
            });
        }

        let next_page = if items.len() as u64 == input.page_size {
            Some(input.page + 1)
        } else {
            None
        };
        let mut message = String::new();
        let summary = format!(
            "Returned {} definitions from {} input path(s). depth={}.{}",
            items.len(),
            input.relative_paths.len(),
            input.depth,
            if next_page.is_some() {
                " More results available via next-page."
            } else {
                ""
            }
        );
        if !message.is_empty() {
            message.push('\n');
        }
        message.push_str(&summary);

        let mut dir_set: HashSet<String> = collected_directories_rel.into_iter().collect();
        if input.show_definitions {
            for abs in &abs_paths {
                if let Some(parent) = Path::new(abs).parent()
                    && let Ok(relp) = parent.strip_prefix(&project_root)
                {
                    dir_set.insert(relp.to_string_lossy().to_string());
                }
            }
        }
        let mut directories_sorted: Vec<String> = dir_set.into_iter().collect();
        directories_sorted.sort();

        let xml = build_repo_map_xml(
            items,
            directories_sorted,
            input.show_directories,
            input.show_definitions,
            next_page,
            input.depth,
            message,
        )
        .map_err(|e| rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(xml)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::index_project::IndexProjectTool;
    use database::kuzu::database::KuzuDatabase;
    use database::querying::DatabaseQueryingService;
    use event_bus::EventBus;
    use rmcp::model::object;
    use serde_json::json;
    use tempfile::TempDir;
    use testing::repository::TestRepository;

    fn setup_ts_workspace() -> (TempDir, TempDir, Arc<WorkspaceManager>, String) {
        let temp_workspace_dir = TempDir::new().unwrap();
        let workspace_path = temp_workspace_dir.path().join("ts_workspace_e2e");
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
        if let Err(e) = &index_result {
            eprintln!("INDEX_DEBUG: indexing error={:?}", e);
        }
        assert!(index_result.is_ok(), "Indexing should succeed");
    }

    fn make_tool(
        database: Arc<KuzuDatabase>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> impl KnowledgeGraphTool {
        RepoMapTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::clone(&workspace_manager),
        )
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_repo_map_typescript_e2e_basic_depth2() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_ts_workspace();
        index_project(&workspace_manager, &project_path).await;

        let database = Arc::new(KuzuDatabase::new());
        // Create tool
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // Run repo_map over the project root with depth=2
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "relative_paths": ["."],
                "depth": 2,
                "page": 1,
                "page_size": 200,
            })))
            .await
            .unwrap();
        let xml = match &result.content.unwrap()[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        eprintln!("TS_REPO_MAP_E2E_XML=\n{}", xml);

        // Verify TypeScript file names and class declarations appear
        assert!(xml.contains("app/models/user_model.ts"));
        assert!(xml.contains("main.ts"));
        assert!(xml.contains("export class BaseModel"));
        assert!(xml.contains("export class UserModel"));
        assert!(xml.contains("class Application"));

        // Directories block should contain ASCII entries
        assert!(xml.contains("<directories>"));
        assert!(xml.contains("app"));
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_repo_map_typescript_e2e_flags_toggle() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_ts_workspace();
        index_project(&workspace_manager, &project_path).await;
        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // Directories only
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "relative_paths": ["."],
                "depth": 2,
                "show_definitions": false,
                "show_directories": true,
            })))
            .await
            .unwrap();
        let xml = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(xml.contains("<directories>"));
        assert!(!xml.contains("<files>"));

        // Definitions only
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "relative_paths": ["."],
                "depth": 2,
                "show_definitions": true,
                "show_directories": false,
            })))
            .await
            .unwrap();
        let xml = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(!xml.contains("<directories>"));
        assert!(xml.contains("<files>"));
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_repo_map_typescript_e2e_depth_respected() {
        let (_ws_tmp, _data_tmp, workspace_manager, project_path) = setup_ts_workspace();
        index_project(&workspace_manager, &project_path).await;
        let database = Arc::new(KuzuDatabase::new());
        let tool: &dyn KnowledgeGraphTool =
            &make_tool(Arc::clone(&database), Arc::clone(&workspace_manager));

        // Depth = 1: nested app/models should be excluded
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "relative_paths": ["."],
                "depth": 1,
                "page": 1,
                "page_size": 50,
            })))
            .await
            .unwrap();
        let xml_d1 = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(!xml_d1.contains("app/models/user_model.ts"));

        // Depth = 2: nested app/models should be included
        let result = tool
            .call(object(json!({
                "project_absolute_path": project_path,
                "relative_paths": ["."],
                "depth": 2,
                "page": 1,
                "page_size": 50,
            })))
            .await
            .unwrap();
        let xml_d2 = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();
        assert!(xml_d2.contains("app/models/user_model.ts"));
    }
}
