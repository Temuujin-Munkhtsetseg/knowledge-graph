use std::{borrow::Cow, cmp::min, path::Path, sync::Arc};

use database::querying::QueryLibrary;
use rmcp::model::{CallToolResult, Content, ErrorCode, Tool, object};
use serde::Serialize;
use serde_json::{Map, Value, json};
use tokio::time::{Duration, timeout};

use crate::tools::{
    file_reader_utils::read_file_chunks,
    types::{KnowledgeGraphTool, KnowledgeGraphToolInput},
    utils::get_database_path,
};
use workspace_manager::WorkspaceManager;

pub const SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME: &str = "search_codebase_definitions";
const SEARCH_CODEBASE_DEFINITIONS_TOOL_DESCRIPTION: &str = r#"Searches for functions, classes, methods, constants, and interfaces in the codebase.

Behavior:
- Finds multiple code definitions using the search terms across all files in the specified project.
- Supports exact and partial matching.
- Returns signatures, locations and the definition type of the matching definitions.
- Large result sets are paginated with the `page` parameter.

Requirements:
- Provide one or multiple search terms to locate the definitions.
- Specify the absolute filesystem path to the project root directory.

Use cases:
- Finding function, class, method, constant, interface definitions across the codebase
- Understanding code structure and architecture
- Getting overview of available APIs and interfaces

Example:
Searching for multiple definitions in a React project:
Search terms: ["useState", "ComponentProps", "handleSubmit"]
Project: "/home/user/my-react-app"
Page: 1 (first page)

Call:
{
  "search_terms": ["useState", "ComponentProps", "handleSubmit"],
  "project": "/home/user/my-react-app",
  "page": 1
}

This will find all definitions matching those names throughout the codebase, returning their signatures and locations.
Tip: Use this tool in combination with get_references tool - first locate definitions with this tool, then use get_references tool to see where they're used throughout the codebase."#;

#[derive(Debug, Clone, Serialize)]
pub struct ResultItem {
    pub name: String,
    pub fqn: String,
    pub definition_type: String,
    pub location: String,
    pub context: Option<String>,
}

#[derive(Debug)]
struct SearchError {
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SearchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| &**e as &(dyn std::error::Error + 'static))
    }
}

impl From<SearchError> for rmcp::ErrorData {
    fn from(err: SearchError) -> Self {
        rmcp::ErrorData::new(ErrorCode::INTERNAL_ERROR, err.message, None)
    }
}

// Configuration constants
const DEFAULT_PAGE: u64 = 1;
const PAGE_SIZE: u64 = 50;
const MIN_PAGE: u64 = 1;

const CONTEXT_DEFINITION_LINES: usize = 3;

const FILE_READ_TIMEOUT_SECONDS: u64 = 10;

#[derive(Serialize)]
pub struct SearchCodebaseDefinitionsToolOutput {
    pub definitions: Vec<ResultItem>,
    pub next_page: Option<u64>,
    pub system_message: String,
}

impl SearchCodebaseDefinitionsToolOutput {
    pub fn empty(system_message: String) -> Self {
        Self {
            definitions: Vec::new(),
            next_page: None,
            system_message,
        }
    }

    pub fn new(
        definitions: Vec<ResultItem>,
        next_page: Option<u64>,
        system_message: String,
    ) -> Self {
        Self {
            definitions,
            next_page,
            system_message,
        }
    }
}

pub struct SearchCodebaseDefinitionsTool {
    pub query_service: Arc<dyn database::querying::QueryingService>,
    pub workspace_manager: Arc<WorkspaceManager>,
}

impl SearchCodebaseDefinitionsTool {
    pub fn new(
        query_service: Arc<dyn database::querying::QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            query_service,
            workspace_manager,
        }
    }

    /// Executes database queries and populates file content in one clean step
    async fn search_and_populate_content(
        &self,
        project_absolute_path: &str,
        database_path: &Path,
        search_terms: &[String],
        page: u64,
    ) -> Result<SearchCodebaseDefinitionsToolOutput, SearchError> {
        // Execute a single database query for all search terms
        let query = QueryLibrary::get_search_definitions_query();
        let mut query_params = Map::new();

        // Convert search terms to lowercase for case-insensitive matching
        let lowercase_terms: Vec<Value> = search_terms
            .iter()
            .map(|term| Value::String(term.to_lowercase()))
            .collect();

        query_params.insert("search_terms".to_string(), Value::Array(lowercase_terms));
        query_params.insert("limit".to_string(), Value::Number(PAGE_SIZE.into()));
        query_params.insert(
            "skip".to_string(),
            Value::Number(((page - 1) * PAGE_SIZE).into()),
        );

        let mut query_result = self
            .query_service
            .execute_query(database_path.to_path_buf(), query.query, query_params)
            .map_err(|e| SearchError {
                message: format!("Database query failed: {}.", e),
                source: None,
            })?;

        let mut query_results = Vec::new();
        while let Some(row) = query_result.next() {
            let name = row.get_string_value(0).unwrap_or_default();
            let fqn = row.get_string_value(1).unwrap_or_default();
            let definition_type = row.get_string_value(2).unwrap_or_default();
            let primary_file_path = row.get_string_value(3).unwrap_or_default();
            let start_line = row.get_int_value(4).unwrap_or(0) as usize;
            let end_line = row.get_int_value(5).unwrap_or(0) as usize;

            query_results.push((
                name,
                fqn,
                definition_type,
                Path::new(project_absolute_path)
                    .join(primary_file_path)
                    .to_string_lossy()
                    .to_string(),
                start_line + 1, // one-indexed
                end_line + 1,   // one-indexed
            ));
        }

        if query_results.is_empty() {
            let system_message = self.get_system_message(
                search_terms,
                project_absolute_path,
                Vec::new(),
                query_results.len(),
                None,
            );
            return Ok(SearchCodebaseDefinitionsToolOutput::empty(system_message));
        }

        // Prepare file chunks to read (with deduplication)
        let file_chunks: Vec<(String, usize, usize)> = query_results
            .iter()
            .map(|(_, _, _, file_path, start_line, end_line)| {
                let context_end = min(*start_line + CONTEXT_DEFINITION_LINES, *end_line);
                (file_path.clone(), *start_line, context_end)
            })
            .collect();

        // Read all files concurrently
        let file_contents = timeout(
            Duration::from_secs(FILE_READ_TIMEOUT_SECONDS),
            read_file_chunks(file_chunks),
        )
        .await
        .map_err(|_| SearchError {
            message: "File reading operation timed out.".to_string(),
            source: None,
        })?
        .map_err(|e| SearchError {
            message: format!("Failed to read file chunks: {}.", e),
            source: None,
        })?;

        let mut file_read_errors = Vec::new();
        // Build final results with content
        let results: Vec<ResultItem> = query_results
            .into_iter()
            .zip(file_contents.into_iter())
            .map(
                |(
                    (name, fqn, definition_type, file_path, start_line, end_line),
                    content_result,
                )| {
                    let context = match content_result {
                        Ok(content) => Some(content.trim().to_string()),
                        Err(_) => {
                            file_read_errors.push(file_path.clone());
                            None
                        }
                    };

                    ResultItem {
                        name,
                        fqn,
                        definition_type,
                        location: format!("{}:L{}-{}", file_path, start_line, end_line),
                        context,
                    }
                },
            )
            .collect();

        let next_page = if results.len() == PAGE_SIZE as usize {
            Some(page + 1)
        } else {
            None
        };
        let system_message = self.get_system_message(
            search_terms,
            project_absolute_path,
            file_read_errors,
            results.len(),
            next_page,
        );

        Ok(SearchCodebaseDefinitionsToolOutput {
            definitions: results,
            next_page,
            system_message,
        })
    }

    fn get_system_message(
        &self,
        search_terms: &[String],
        project_absolute_path: &str,
        file_read_errors: Vec<String>,
        results_count: usize,
        next_page: Option<u64>,
    ) -> String {
        let mut message = String::new();

        for (index, file_read_error) in file_read_errors.iter().enumerate() {
            if index == 0 {
                message.push_str("Failed to read some some files:");
            }
            message.push_str(&format!("\n- {}.", file_read_error));
            if index == file_read_errors.len() - 1 {
                message.push_str(
                    "\nPerhaps some files were deleted, moved or changed since the last indexing.",
                );
                message.push_str(&format!("\nIf the missing context is important, use the `index_project` tool to re-index the project {} and try again.\n", project_absolute_path));
            }
        }

        if results_count > 0 {
            message.push_str(&format!(
                "Found a total of {} definitions for the search terms ({}) in the project {}.\n",
                results_count,
                search_terms.join(", "),
                project_absolute_path
            ));

            message.push_str(r#"
                Decision Framework:
                - If sufficient context for your current task is provided in the results, you can stop here.
                - If you've found a definition you want to examine further, use the `get_references` tool to examine the references to the relevant symbol.
                - If you've found a definition you want to read the implementation of, use the `read_definitions` tool to read the implementation.
                - If the results revealed a new relevant symbol, use the `search_codebase_definitions` tool again with different search terms to explore further.
            "#);
        } else if results_count == 0 {
            message.push_str(&format!(
                "No indexed definitions found for the search terms ({}) in the project {}.\n",
                search_terms.join(", "),
                project_absolute_path
            ));

            message.push_str(r#"
                Decision Framework:
                - If you know for sure that definitions exists for the search terms, you can use the `index_project` tool to re-index the project and try again.
                - If you know for sure that definitions exists for the search terms, and the indexing is up to date, you can stop using the Knowledge Graph for getting definitions for the requested search terms.
            "#);
        }

        if let Some(next_page) = next_page {
            message.push_str(&format!(
                "There are more results on page {} if more context is needed for the current task.",
                next_page
            ));
        }

        message
    }
}

impl KnowledgeGraphTool for SearchCodebaseDefinitionsTool {
    fn name(&self) -> &str {
        SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let all_projects_paths = self
            .workspace_manager
            .list_all_projects()
            .iter()
            .map(|project| project.project_path.clone())
            .collect::<Vec<_>>()
            .join(",");

        let input_schema = json!({
            "type": "object",
            "properties": {
                "search_terms": {
                    "type": "array",
                    "description": "List of definition names to search for. Can be names of functions, classes, constants, etc.",
                    "items": {
                        "type": "string"
                    }
                },
                "project": {
                    "type": "string",
                    "description": "Absolute filesystem path to the project root directory where code definitions should be searched.",
                    "enum": [all_projects_paths]
                },
                "page": {
                    "type": "number",
                    "description": "Page number starting from 1. If the response's next_page field is greater than 1, more results are available at that page. You can use this to retrieve more results if more context is needed.",
                    "default": DEFAULT_PAGE,
                    "minimum": MIN_PAGE,
                }
            },
            "required": ["search_terms", "project"],
        });

        Tool {
            name: Cow::Borrowed(SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME),
            description: Some(Cow::Borrowed(SEARCH_CODEBASE_DEFINITIONS_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    fn call(
        &self,
        params: rmcp::model::JsonObject,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let input = KnowledgeGraphToolInput { params };

        // Extract and validate parameters with better error messages
        let search_terms = input.get_string_array("search_terms")?;
        let project_absolute_path = input.get_string("project")?;
        let page = input.get_u64("page").unwrap_or(DEFAULT_PAGE).max(MIN_PAGE);

        let database_path = get_database_path(&self.workspace_manager, project_absolute_path)?;

        let output = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.search_and_populate_content(
                project_absolute_path,
                &database_path,
                &search_terms,
                page,
            ))
        })
        .map_err(rmcp::ErrorData::from)?;

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

    use crate::tools::{SearchCodebaseDefinitionsTool, types::KnowledgeGraphTool};

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_search_codebase_definitions_context_lines() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &SearchCodebaseDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        let project = &setup.workspace_manager.clone().list_all_projects()[0];

        let result = tool
            .call(object(json!({
                "project": project.project_path.clone(),
                "search_terms": ["main"],
                "page": 1,
            })))
            .unwrap();

        let content = result.content.expect("Expected content in result");
        assert_eq!(content.len(), 1, "Expected single JSON object in response");

        let content_item = &content[0];
        let rmcp::model::Annotated { raw, .. } = content_item;
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        assert!(
            parsed.get("definitions").is_some(),
            "Expected 'definitions' field"
        );
        assert!(
            parsed.get("system_message").is_some(),
            "Expected 'system_message' field"
        );
        assert_eq!(
            parsed["next_page"],
            json!(null),
            "Expected 'next_page' to be null"
        );

        let definitions = parsed["definitions"].as_array().unwrap();
        assert!(!definitions.is_empty(), "Expected non-empty definitions");

        for definition in definitions {
            let name = definition["name"].as_str().unwrap();
            let fqn = definition["fqn"].as_str().unwrap();
            let definition_type = definition["definition_type"].as_str().unwrap();
            let location = definition["location"].as_str().unwrap();
            let context = definition["context"].as_str().unwrap();

            match (name, definition_type) {
                ("Main", "Constructor") => {
                    assert_eq!(fqn, "com.example.app.Main.Main");
                    assert!(location.contains("Main.java:L11-13"));
                    assert!(
                        context.contains("myParameter = new Foo()")
                            && context.contains("public Main() {")
                            && context.contains("}")
                    );
                }
                ("Main", "Class") => {
                    assert_eq!(fqn, "com.example.app.Main");
                    assert!(location.contains("Main.java:L8-49"));
                    assert!(context.contains("public class Main extends Application"));
                }
                ("main", "Method") => {
                    assert_eq!(fqn, "com.example.app.Main.main");
                    assert!(location.contains("Main.java:L15-44"));
                    assert!(
                        context.contains("@Traceable") && context.contains("public void main()")
                    );
                }
                ("await", "Method") => {
                    assert_eq!(fqn, "com.example.app.Main.await");
                    assert!(location.contains("Main.java:L46-48"));
                    assert!(context.contains("fn.run()"));
                }
                _ => {
                    panic!("Unexpected result: {:?}", (name, definition_type));
                }
            }
        }

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_search_codebase_definitions_has_next_page() {
        let database = Arc::new(KuzuDatabase::new());
        let setup = setup_java_reference_pipeline(&database).await;

        database
            .get_or_create_database(&setup.database_path, None)
            .expect("Failed to create database");

        let tool: &dyn KnowledgeGraphTool = &SearchCodebaseDefinitionsTool::new(
            Arc::new(DatabaseQueryingService::new(database)),
            Arc::new(setup.workspace_manager.clone()),
        );

        let project = &setup.workspace_manager.clone().list_all_projects()[0];

        let first_page_result = tool
            .call(object(json!({
                "project": project.project_path.clone(),
                "search_terms": ["repeatedMethod"],
                "page": 1,
            })))
            .unwrap();

        let content = first_page_result
            .content
            .expect("Expected content in result");
        assert_eq!(content.len(), 1, "Expected single JSON object in response");

        let content_item = &content[0];
        let rmcp::model::Annotated { raw, .. } = content_item;
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        assert!(
            parsed.get("definitions").is_some(),
            "Expected 'definitions' field"
        );
        assert!(
            parsed.get("system_message").is_some(),
            "Expected 'system_message' field"
        );

        let definitions = parsed["definitions"].as_array().unwrap();
        assert_eq!(
            definitions.len(),
            50,
            "Expected 50 definitions on first page"
        ); // 60 repeatedMethod definitions, 50 per page

        let next_page = parsed["next_page"].as_u64().unwrap();
        assert_eq!(next_page, 2);

        let second_page_result = tool
            .call(object(json!({
                "project": project.project_path.clone(),
                "search_terms": ["repeatedMethod"],
                "page": 2,
            })))
            .unwrap();

        let content = second_page_result
            .content
            .expect("Expected content in result");
        assert_eq!(content.len(), 1, "Expected single JSON object in response");

        let content_item = &content[0];
        let rmcp::model::Annotated { raw, .. } = content_item;
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        let definitions = parsed["definitions"].as_array().unwrap();
        assert_eq!(
            definitions.len(),
            10,
            "Expected 10 definitions on second page"
        ); // 60 total, 50 on first page, 10 remaining

        assert!(
            parsed["next_page"].is_null(),
            "Expected 'next_page' to be null on last page"
        );

        setup.cleanup();
    }
}
