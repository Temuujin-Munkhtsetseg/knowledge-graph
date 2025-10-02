use std::{borrow::Cow, cmp::min, path::Path, sync::Arc};

use crate::tools::xml::{ToXml, XmlBuilder};
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

impl ToXml for SearchCodebaseDefinitionsToolOutput {
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut builder = XmlBuilder::new();

        builder.start_element("ToolResponse")?;

        builder.start_element("definitions")?;
        for definition in &self.definitions {
            builder.start_element("definition")?;
            builder.write_element("name", &definition.name)?;
            builder.write_element("fqn", &definition.fqn)?;
            builder.write_element("definition-type", &definition.definition_type)?;
            builder.write_element("location", &definition.location)?;
            builder.write_optional_cdata_element("context", &definition.context)?;
            builder.end_element("definition")?;
        }
        builder.end_element("definitions")?;

        builder.write_optional_numeric_element("next-page", &self.next_page)?;

        builder.write_cdata_element("system-message", &self.system_message)?;

        builder.end_element("ToolResponse")?;
        builder.finish()
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
                message: format!("Database query failed: {e}."),
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
            message: format!("Failed to read file chunks: {e}."),
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
                        location: format!("{file_path}:L{start_line}-{end_line}"),
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
            message.push_str(&format!("\n- {file_read_error}."));
            if index == file_read_errors.len() - 1 {
                message.push_str(
                    "\nPerhaps some files were deleted, moved or changed since the last indexing.",
                );
                message.push_str(&format!("\nIf the missing context is important, use the `index_project` tool to re-index the project {project_absolute_path} and try again.\n"));
            }
        }

        if results_count > 0 {
            message.push_str(&format!(
                "Found a total of {} definitions for the search terms ({}) in the project {}.\n",
                results_count,
                search_terms.join(", "),
                project_absolute_path
            ));

            message.push_str("Decision Framework:\n");
            message.push_str("  - If sufficient context for your current task is provided in the results, you can stop here.\n");
            message.push_str("  - If you've found a definition you want to examine further, use the `get_references` tool to examine the references to the relevant symbol.\n");
            message.push_str("  - If you've found a definition you want to read the implementation of, use the `read_definitions` tool to read the implementation.\n");
            message.push_str("  - If the results revealed a new relevant symbol, use the `search_codebase_definitions` tool again with different search terms to explore further.\n");
        } else if results_count == 0 {
            message.push_str(&format!(
                "No indexed definitions found for the search terms ({}) in the project {}.\n",
                search_terms.join(", "),
                project_absolute_path
            ));

            message.push_str("Decision Framework:\n");
            message.push_str("  - If you know for sure that definitions exists for the search terms, you can use the `index_project` tool to re-index the project and try again.\n");
            message.push_str("  - If you know for sure that definitions exists for the search terms, and the indexing is up to date, you can stop using the Knowledge Graph for getting definitions for the requested search terms.\n");
        }

        if let Some(next_page) = next_page {
            message.push_str(&format!(
                "There are more results on page {next_page} if more context is needed for the current task."
            ));
        }

        message
    }
}

#[async_trait::async_trait]
impl KnowledgeGraphTool for SearchCodebaseDefinitionsTool {
    fn name(&self) -> &str {
        SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
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
                "project_absolute_path": {
                    "type": "string",
                    "description": "Absolute filesystem path to the project root directory where code definitions should be searched. You can use the list_projects tool to get the list of indexed projects.",
                },
                "page": {
                    "type": "number",
                    "description": "Page number starting from 1. If the response's next_page field is greater than 1, more results are available at that page. You can use this to retrieve more results if more context is needed.",
                    "default": DEFAULT_PAGE,
                    "minimum": MIN_PAGE,
                }
            },
            "required": ["search_terms", "project_absolute_path"],
        });

        Tool {
            name: Cow::Borrowed(SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME),
            description: Some(Cow::Borrowed(SEARCH_CODEBASE_DEFINITIONS_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    async fn call(
        &self,
        params: rmcp::model::JsonObject,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let input = KnowledgeGraphToolInput { params };

        // Extract and validate parameters with better error messages
        let search_terms = input.get_string_array("search_terms")?;
        let project_absolute_path = input.get_string("project_absolute_path")?;
        let page = input.get_u64("page").unwrap_or(DEFAULT_PAGE).max(MIN_PAGE);

        let database_path = get_database_path(&self.workspace_manager, project_absolute_path)?;

        let output = self
            .search_and_populate_content(project_absolute_path, &database_path, &search_terms, page)
            .await
            .map_err(rmcp::ErrorData::from)?;

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
                "project_absolute_path": project.project_path.clone(),
                "search_terms": ["main"],
                "page": 1,
            })))
            .await
            .unwrap();

        let content = result.content.expect("Expected content in result");
        assert_eq!(content.len(), 1, "Expected single JSON object in response");

        let content_item = &content[0];
        let rmcp::model::Annotated { raw, .. } = content_item;
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
            xml_str.contains("<system-message>"),
            "Expected system-message element"
        );
        assert!(
            !xml_str.contains("<next-page>"),
            "Expected no next-page element"
        );
        assert!(
            xml_str.contains("<definition>"),
            "Expected at least one definition"
        );

        // Verify specific definitions are present
        if xml_str.contains("<name>Main</name>")
            && xml_str.contains("<definition-type>Constructor</definition-type>")
        {
            assert!(
                xml_str.contains("<fqn>com.example.app.Main.Main</fqn>"),
                "Expected Main constructor fqn"
            );
            assert!(
                xml_str.contains("Main.java:L11-13"),
                "Expected Main constructor location"
            );
            assert!(
                xml_str.contains("public Main() {"),
                "Expected constructor signature in context"
            );
            assert!(
                xml_str.contains("myParameter = new Foo()"),
                "Expected constructor context"
            );
            assert!(xml_str.contains("}"), "Expected closing brace in context");
        }

        // Check for other expected definitions
        if xml_str.contains("<name>Main</name>")
            && xml_str.contains("<definition-type>Class</definition-type>")
        {
            assert!(
                xml_str.contains("<fqn>com.example.app.Main</fqn>"),
                "Expected Main class fqn"
            );
            assert!(
                xml_str.contains("public class Main extends Application"),
                "Expected class context"
            );
        }

        if xml_str.contains("<name>main</name>")
            && xml_str.contains("<definition-type>Method</definition-type>")
        {
            assert!(
                xml_str.contains("<fqn>com.example.app.Main.main</fqn>"),
                "Expected main method fqn"
            );
            assert!(xml_str.contains("@Traceable"), "Expected method annotation");
            assert!(
                xml_str.contains("public void main() {"),
                "Expected method signature in context"
            );
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
                "project_absolute_path": project.project_path.clone(),
                "search_terms": ["repeatedMethod"],
                "page": 1,
            })))
            .await
            .unwrap();

        let content = first_page_result
            .content
            .expect("Expected content in result");
        assert_eq!(content.len(), 1, "Expected single JSON object in response");

        let content_item = &content[0];
        let rmcp::model::Annotated { raw, .. } = content_item;
        let xml_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure for pagination test (first page)
        assert!(
            xml_str.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str.contains("<definitions>"),
            "Expected definitions element"
        );
        assert!(
            xml_str.contains("<system-message>"),
            "Expected system-message element"
        );

        let definition_count = xml_str.matches("<definition>").count();
        assert_eq!(
            definition_count, 50,
            "Expected 50 definitions on first page"
        );

        assert!(
            xml_str.contains("<next-page>2</next-page>"),
            "Expected next-page element with value 2"
        );

        let second_page_result = tool
            .call(object(json!({
                "project_absolute_path": project.project_path.clone(),
                "search_terms": ["repeatedMethod"],
                "page": 2,
            })))
            .await
            .unwrap();

        let content = second_page_result
            .content
            .expect("Expected content in result");
        assert_eq!(content.len(), 1, "Expected single JSON object in response");

        let content_item = &content[0];
        let rmcp::model::Annotated { raw, .. } = content_item;
        let xml_str = match raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => panic!("Expected text content"),
        };

        // Parse and validate XML structure for pagination test
        assert!(
            xml_str.contains("<ToolResponse>"),
            "Expected ToolResponse root element"
        );
        assert!(
            xml_str.contains("<definitions>"),
            "Expected definitions element"
        );
        let definition_count = xml_str.matches("<definition>").count();
        assert_eq!(
            definition_count, 10,
            "Expected 10 definitions on second page"
        );
        assert!(
            !xml_str.contains("<next-page>"),
            "Expected no next-page element on last page"
        );

        setup.cleanup();
    }
}
