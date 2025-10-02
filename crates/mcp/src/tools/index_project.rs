use std::sync::Arc;
use std::thread;
use std::{borrow::Cow, collections::HashMap};

use crate::tools::xml::{ToXml, XmlBuilder};
use database::kuzu::database::KuzuDatabase;
use event_bus::EventBus;
use indexer::execution::{config::IndexingConfigBuilder, executor::IndexingExecutor};
use indexer::stats::ProjectStatistics;
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, Tool, object};
use serde::Serialize;
use serde_json::json;
use tokio::runtime::Builder;
use workspace_manager::WorkspaceManager;

use crate::tools::types::{KnowledgeGraphTool, KnowledgeGraphToolInput};

pub const INDEX_PROJECT_TOOL_NAME: &str = "index_project";
const INDEX_PROJECT_TOOL_DESCRIPTION: &str = r#"Rebuild the Knowledge Graph index to reflect recent changes in the project.

Behavior:
- Scans the entire project and regenerates the Knowledge Graph from scratch.
- Updates all file relationships, dependencies, and cross-references.

Requirements:
- Specify the absolute filesystem path to the project root directory.
- The project must be indexed in the Knowledge Graph.

When to use:
- After substantial file modifications, additions, or deletions.
- When the Knowledge Graph appears stale or incomplete.

Example:
Call:
{ "project": "/path/to/project" }
"#;

#[derive(Serialize)]
pub struct IndexProjectToolOutput {
    pub stats: IndexProjectToolStatsOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
}

impl From<ProjectStatistics> for IndexProjectToolStatsOutput {
    fn from(project_statistics: ProjectStatistics) -> Self {
        Self {
            project_path: project_statistics.project_path,
            total_files: project_statistics.total_files,
            total_definitions: project_statistics.total_definitions,
            total_imported_symbols: project_statistics.total_imported_symbols,
            total_definition_relationships: project_statistics.total_definition_relationships,
            total_imported_symbol_relationships: project_statistics
                .total_imported_symbol_relationships,
            languages: project_statistics
                .languages
                .into_iter()
                .map(|language| IndexProjectToolLanguageStatsOutput {
                    language: language.language,
                    file_count: language.file_count,
                    definition_count: language.definitions_count,
                    definition_type_counts: language.definition_type_counts,
                })
                .collect(),
            indexing_duration_seconds: project_statistics.indexing_duration_seconds,
        }
    }
}

#[derive(Serialize)]
pub struct IndexProjectToolStatsOutput {
    pub project_path: String,
    pub total_files: usize,
    pub total_definitions: usize,
    pub total_imported_symbols: usize,
    pub total_definition_relationships: usize,
    pub total_imported_symbol_relationships: usize,
    pub languages: Vec<IndexProjectToolLanguageStatsOutput>,
    pub indexing_duration_seconds: f64,
}

#[derive(Serialize)]
pub struct IndexProjectToolLanguageStatsOutput {
    pub language: String,
    pub file_count: usize,
    pub definition_count: usize,
    pub definition_type_counts: HashMap<String, usize>,
}

impl ToXml for IndexProjectToolOutput {
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut builder = XmlBuilder::new();

        builder.start_element("ToolResponse")?;

        builder.start_element("stats")?;
        builder.write_element("project-path", &self.stats.project_path)?;
        builder.write_numeric_element("total-files", self.stats.total_files)?;
        builder.write_numeric_element("total-definitions", self.stats.total_definitions)?;
        builder
            .write_numeric_element("total-imported-symbols", self.stats.total_imported_symbols)?;
        builder.write_numeric_element(
            "total-definition-relationships",
            self.stats.total_definition_relationships,
        )?;
        builder.write_numeric_element(
            "total-imported-symbol-relationships",
            self.stats.total_imported_symbol_relationships,
        )?;
        builder.write_numeric_element(
            "indexing-duration-seconds",
            self.stats.indexing_duration_seconds,
        )?;

        builder.start_element("languages")?;
        for language in &self.stats.languages {
            builder.start_element("language")?;
            builder.write_element("language", &language.language)?;
            builder.write_numeric_element("file-count", language.file_count)?;
            builder.write_numeric_element("definition-count", language.definition_count)?;

            builder.start_element("definition-type-counts")?;
            for (def_type, count) in &language.definition_type_counts {
                builder.start_element("definition-type")?;
                builder.write_element("type", def_type)?;
                builder.write_numeric_element("count", *count)?;
                builder.end_element("definition-type")?;
            }
            builder.end_element("definition-type-counts")?;

            builder.end_element("language")?;
        }
        builder.end_element("languages")?;

        builder.end_element("stats")?;

        builder.write_optional_cdata_element("system-message", &self.system_message)?;

        builder.end_element("ToolResponse")?;
        builder.finish()
    }
}

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

    fn get_system_message(&self, project_stats: &ProjectStatistics) -> Option<String> {
        if project_stats.total_definitions == 0 {
            let mut message = String::new();
            message.push_str(&format!(
                "The Knowledge Graph failed to index any definitions in the project {}.",
                project_stats.project_path
            ));
            message.push_str("This means that the Knowledge Graph is unable to provide useful information for this project and using its tools will not be useful for your current task.");
            return Some(message);
        }

        None
    }
}

#[async_trait::async_trait]
impl KnowledgeGraphTool for IndexProjectTool {
    fn name(&self) -> &str {
        INDEX_PROJECT_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let input_schema = json!({
            "type": "object",
            "properties": {
                "project_absolute_path": {
                    "type": "string",
                    "description": "Absolute filesystem path to the project root directory to re-index. You can use the list_projects tool to get the list of indexed projects.",
                }
            },
            "required": ["project_absolute_path"]
        });

        Tool {
            name: Cow::Borrowed(INDEX_PROJECT_TOOL_NAME),
            description: Some(Cow::Borrowed(INDEX_PROJECT_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    async fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let input = KnowledgeGraphToolInput { params };

        let project_absolute_path = input.get_string("project_absolute_path")?;

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

        let system_message = self.get_system_message(&project_stats);
        let output = IndexProjectToolOutput {
            stats: project_stats.into(),
            system_message,
        };

        let xml_output = output.to_xml_without_cdata().map_err(|e| {
            rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Failed to convert output to XML: {e}"),
                None,
            )
        })?;

        Ok(CallToolResult::success(vec![Content::text(xml_output)]))
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

        let result =
            futures::executor::block_on(tool.call(params)).expect("tool call should succeed");
        let text = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();

        // Parse and validate XML structure
        assert!(
            text.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(text.contains("<stats>"), "Expected stats element");
        assert!(
            text.contains("<project-path>"),
            "Expected project-path element"
        );
        assert!(
            text.contains(&format!("<project-path>{project_path}</project-path>")),
            "Expected project path in XML"
        );
        assert!(
            text.contains("<total-files>"),
            "Expected total-files element"
        );
        assert!(
            text.contains("<total-definitions>"),
            "Expected total-definitions element"
        );
    }
}
