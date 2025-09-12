use std::{borrow::Cow, cmp::min, path::Path, sync::Arc};

use database::querying::QueryLibrary;
use rmcp::model::{CallToolResult, Content, ErrorCode, Tool, object};
use serde_json::{Map, Value, json};
use tokio::time::{Duration, timeout};

use crate::tools::{
    file_reader_utils::read_file_chunks,
    types::{KnowledgeGraphTool, KnowledgeGraphToolInput},
    utils::get_database_path,
};
use workspace_manager::WorkspaceManager;

pub const SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME: &str = "search_codebase_definitions";
const SEARCH_CODEBASE_DEFINITIONS_TOOL_DESCRIPTION: &str = "
Searches for functions, classes, methods, constants, and interfaces in the codebase. Returns definition signatures and optionally full implementations. Supports exact/partial matching and case-sensitive search.

Best practice: First search for signatures only to get an overview, then request full implementations for specific items as needed.

Use for: Finding definitions, understanding code structure, debugging, locating usages, and refactoring.
";

#[derive(Debug, Clone)]
pub struct ResultItem {
    pub name: String,
    pub fqn: String,
    pub definition_type: String,
    pub location: String,
    pub context: Option<String>,
    pub body: Option<String>,
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
const DEFAULT_INCLUDE_FULL_BODY: bool = false;
const MIN_PAGE: u64 = 1;

const CONTEXT_DEFINITION_LINES: usize = 3;

const FILE_READ_TIMEOUT_SECONDS: u64 = 10;

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
        include_full_body: bool,
    ) -> Result<Vec<ResultItem>, SearchError> {
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
            return Ok(Vec::new());
        }

        // Prepare file chunks to read (with deduplication)
        let file_chunks: Vec<(String, usize, usize)> = query_results
            .iter()
            .map(|(_, _, _, file_path, start_line, end_line)| {
                if include_full_body {
                    (file_path.clone(), *start_line, *end_line)
                } else {
                    let context_end = min(*start_line + CONTEXT_DEFINITION_LINES, *end_line);
                    (file_path.clone(), *start_line, context_end)
                }
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

        // Build final results with content
        let results: Vec<ResultItem> = query_results
            .into_iter()
            .zip(file_contents.into_iter())
            .map(
                |(
                    (name, fqn, definition_type, file_path, start_line, end_line),
                    content_result,
                )| {
                    let file_content = content_result.ok().map(|c| c.trim().to_string());
                    ResultItem {
                        name,
                        fqn,
                        definition_type,
                        location: format!("{}:L{}-{}", file_path, start_line, end_line),
                        context: if include_full_body {
                            None
                        } else {
                            file_content.clone()
                        },
                        body: if include_full_body {
                            file_content
                        } else {
                            None
                        },
                    }
                },
            )
            .collect();

        Ok(results)
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
                "include_full_body": {
                    "type": "boolean",
                    "description": "If true, returns implementations of each definition. If false, returns only the definition signatures. Best practice: Use false to get a broad overview, then use true to examine a definition more closely.",
                    "default": DEFAULT_INCLUDE_FULL_BODY
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

        let include_full_body = input
            .get_boolean("include_full_body")
            .unwrap_or(DEFAULT_INCLUDE_FULL_BODY);

        let database_path = get_database_path(&self.workspace_manager, project_absolute_path)?;

        let results = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.search_and_populate_content(
                project_absolute_path,
                &database_path,
                &search_terms,
                page,
                include_full_body,
            ))
        })
        .map_err(rmcp::ErrorData::from)?;

        // Convert results to JSON content
        let mut content = Vec::with_capacity(results.len());
        for item in results {
            let json_value = json!({
                "name": item.name,
                "fqn": item.fqn,
                "definition_type": item.definition_type,
                "location": item.location,
                "context": item.context,
                "body": item.body
            });

            content.push(Content::json(json_value).map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to serialize result to JSON: {}.", e),
                    None,
                )
            })?);
        }

        let has_next_page = content.len() == PAGE_SIZE as usize;
        content.push(
            Content::json(json!({
                "next_page": if has_next_page { json!(page + 1) } else { json!(null) },
            }))
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to serialize result to JSON: {}.", e),
                    None,
                )
            })?,
        );

        Ok(CallToolResult::success(content))
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
                "include_full_body": false,
                "page": 1,
            })))
            .unwrap();

        let content = result.content.expect("Expected content in result");
        assert!(!content.is_empty(), "Expected non-empty content");

        for content_item in &content {
            let rmcp::model::Annotated { raw, .. } = content_item;
            let json_str = match raw {
                rmcp::model::RawContent::Text(text_content) => &text_content.text,
                _ => continue,
            };

            let parsed: serde_json::Value =
                serde_json::from_str(json_str).expect("Expected valid JSON in content");

            if let Some(next_page) = parsed.get("next_page") {
                assert_eq!(next_page, &json!(null));
                continue;
            }

            let name = parsed["name"].as_str().unwrap();
            let fqn = parsed["fqn"].as_str().unwrap();
            let definition_type = parsed["definition_type"].as_str().unwrap();
            let location = parsed["location"].as_str().unwrap();
            let context = parsed["context"].as_str().unwrap();

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

            assert!(
                parsed["body"].is_null(),
                "Expected 'body' to be null when include_full_body is false"
            );
        }

        setup.cleanup();
    }

    #[tracing_test::traced_test]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_search_codebase_definitions_full_body_inclusion() {
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
                "search_terms": ["Foo"],
                "include_full_body": true,
                "page": 1,
            })))
            .unwrap();

        let content = result.content.expect("Expected content in result");

        let code_content_item = content[0].clone();
        let rmcp::model::Annotated { raw, .. } = code_content_item;
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text.clone(),
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        let name = parsed["name"].as_str().unwrap();
        let fqn = parsed["fqn"].as_str().unwrap();
        let definition_type = parsed["definition_type"].as_str().unwrap();
        let location = parsed["location"].as_str().unwrap();
        let body = parsed["body"].as_str().unwrap();

        match (name, definition_type) {
            ("Foo", "Class") => {
                assert_eq!(fqn, "com.example.app.Foo");
                assert!(location.contains("Foo.java:L3-9"));

                assert!(
                    body.contains("public class Foo {"),
                    "Expected class declaration"
                );
                assert!(
                    body.contains("public Executor executor = new Executor();"),
                    "Expected executor field"
                );
                assert!(
                    body.contains("public Bar bar() {"),
                    "Expected bar method declaration"
                );
                assert!(
                    body.contains("return new Bar();"),
                    "Expected bar method implementation"
                );
                assert!(body.contains("}"), "Expected closing brace");

                assert!(
                    parsed["context"].is_null(),
                    "Expected 'context' to be null when include_full_body is true"
                );
            }
            _ => {
                // allow other results (like executor field) but don't require them
            }
        }

        let has_next_page = content[1].clone();
        let rmcp::model::Annotated { raw, .. } = has_next_page;
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text.clone(),
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        assert!(
            parsed["next_page"].is_null(),
            "Expected 'next_page' to be null"
        );

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
                "include_full_body": false,
                "page": 1,
            })))
            .unwrap();

        let content = first_page_result
            .content
            .expect("Expected content in result");
        assert_eq!(content.len(), 51); // 60 repeatedMethod definitions, 50 per page, this is the first page

        let has_next_page = content.last().unwrap().clone();
        let rmcp::model::Annotated { raw, .. } = has_next_page;
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text.clone(),
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        let next_page = parsed["next_page"].as_u64().unwrap();
        assert_eq!(next_page, 2);

        let second_page_result = tool
            .call(object(json!({
                "project": project.project_path.clone(),
                "search_terms": ["repeatedMethod"],
                "include_full_body": false,
                "page": 2,
            })))
            .unwrap();

        let content = second_page_result
            .content
            .expect("Expected content in result");
        assert_eq!(content.len(), 11); // 60 repeatedMethod definitions, 50 per page, this is the last page

        let has_next_page = content.last().unwrap().clone();
        let rmcp::model::Annotated { raw, .. } = has_next_page;
        let json_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text.clone(),
            _ => panic!("Expected text content"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("Expected valid JSON in content");

        assert!(
            parsed["next_page"].is_null(),
            "Expected 'next_page' to be null"
        );

        setup.cleanup();
    }
}
