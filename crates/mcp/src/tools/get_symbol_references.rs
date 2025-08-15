use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use database::kuzu::service::NodeDatabaseService;
use database::kuzu::types::DefinitionNodeFromKuzu;
use database::kuzu::{database::KuzuDatabase, types::KuzuNodeType};
use database::querying::{QueryLibrary, QueryingService};
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, Tool};
use serde_json::{Value, json};
use workspace_manager::WorkspaceManager;

use crate::tools::types::KnowledgeGraphTool;

pub const GET_SYMBOL_REFERENCES_TOOL_NAME: &str = "get_symbol_references";
const GET_SYMBOL_REFERENCES_TOOL_DESCRIPTION: &str = "Finds all locations where a symbol is referenced throughout the codebase to assess change impact. Use this tool when: \
- Planning to modify, rename, or delete a function, class, variable, or other symbol \
- Need to understand the blast radius of a potential change before implementing it \
- Investigating which parts of the codebase depend on a specific symbol \
- Performing impact analysis for refactoring or deprecation decisions \
- Tracing usage patterns to understand how a symbol is being used across the project";

#[derive(Debug)]
pub struct ReferenceInfo {
    pub name: String,
    pub location: String,
    pub fqn: String,
    pub referenced_by: Vec<ReferenceInfo>,
}

impl ReferenceInfo {
    fn to_json(&self) -> Value {
        let mut referenced_by = Vec::new();
        for reference in &self.referenced_by {
            referenced_by.push(reference.to_json());
        }

        json!({
            "name": self.name,
            "location": self.location,
            "fqn": self.fqn,
            "referenced_by": referenced_by
        })
    }
}

pub struct GetSymbolReferencesTool {
    database: Arc<KuzuDatabase>,
    querying_service: Arc<dyn QueryingService>,
    workspace_manager: Arc<WorkspaceManager>,
}

impl GetSymbolReferencesTool {
    pub fn new(
        database: Arc<KuzuDatabase>,
        querying_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            database,
            querying_service,
            workspace_manager,
        }
    }

    fn find_references_recursive(
        &self,
        service: &NodeDatabaseService,
        fqn: &str,
        current_depth: u8,
        max_depth: u8,
        limit: u32,
        visited: &mut std::collections::HashSet<String>,
    ) -> Option<ReferenceInfo> {
        if visited.contains(fqn) || current_depth > max_depth {
            return None;
        }

        visited.insert(fqn.to_string());

        let symbol_info = self.get_symbol_info_from_fqn(service, fqn)?;

        // Find callers of the current symbol
        let callers = service.find_n_first_calls_to_method(fqn, limit);
        if callers.is_err() {
            return None;
        }

        let mut references = Vec::new();
        if current_depth < max_depth {
            for caller_fqn in &callers.unwrap() {
                let caller_ref = self.find_references_recursive(
                    service,
                    caller_fqn,
                    current_depth + 1,
                    max_depth,
                    limit,
                    visited,
                );

                if let Some(caller_ref) = caller_ref {
                    references.push(caller_ref);
                }
            }
        }

        Some(ReferenceInfo {
            name: symbol_info.name,
            fqn: fqn.to_string(),
            location: format!("{}:{}", symbol_info.file, symbol_info.line),
            referenced_by: references,
        })
    }

    fn get_symbol_info_from_fqn(
        &self,
        service: &NodeDatabaseService,
        fqn: &str,
    ) -> Option<SymbolInfo> {
        let nodes = service.get_by::<_, DefinitionNodeFromKuzu>(
            KuzuNodeType::DefinitionNode,
            "fqn",
            &[fqn],
        );

        if let Ok(nodes) = nodes
            && let Some(node) = nodes.first()
        {
            return Some(SymbolInfo {
                name: node.name.clone(),
                fqn: node.fqn.clone(),
                file: node.primary_file_path.clone(),
                line: node.start_line as u32,
            });
        }

        None
    }

    fn search_initial_symbols(
        &self,
        name_or_fqn: &str,
        file_path: &str,
        database_path: PathBuf,
    ) -> Vec<SymbolInfo> {
        let query = QueryLibrary::get_definitions_by_fqn_or_name_query();

        let mut query_params = serde_json::Map::new();
        query_params.insert(
            "file_path".to_string(),
            Value::String(file_path.to_string()),
        );
        query_params.insert(
            "name_or_fqn".to_string(),
            Value::String(name_or_fqn.to_string()),
        );

        let result = self
            .querying_service
            .execute_query(database_path, query.query, query_params);

        if let Ok(mut nodes) = result {
            let mut symbols = Vec::new();

            while let Some(node) = nodes.next() {
                symbols.push(SymbolInfo {
                    // 0 is id
                    name: node.get_string_value(1).unwrap(),
                    fqn: node.get_string_value(2).unwrap(),
                    file: node.get_string_value(3).unwrap(),
                    line: node.get_int_value(4).unwrap() as u32,
                });
            }

            return symbols;
        }

        vec![]
    }
}

#[derive(Debug)]
struct SymbolInfo {
    name: String,
    fqn: String,
    file: String,
    line: u32,
}

impl KnowledgeGraphTool for GetSymbolReferencesTool {
    fn name(&self) -> &str {
        GET_SYMBOL_REFERENCES_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let mut properties = JsonObject::new();

        // absolute_file_path parameter
        let mut file_path_property = JsonObject::new();
        file_path_property.insert("type".to_string(), Value::String("string".to_string()));
        file_path_property.insert(
            "description".to_string(),
            Value::String("The absolute path to the file containing the symbol".to_string()),
        );
        properties.insert(
            "absolute_file_path".to_string(),
            Value::Object(file_path_property),
        );

        // symbol parameter
        let mut symbol_property = JsonObject::new();
        symbol_property.insert("type".to_string(), Value::String("string".to_string()));
        symbol_property.insert(
            "description".to_string(),
            Value::String("The name of the symbol to find references for".to_string()),
        );
        properties.insert("symbol_name".to_string(), Value::Object(symbol_property));

        // depth parameter
        let mut depth_property = JsonObject::new();
        depth_property.insert("type".to_string(), Value::String("integer".to_string()));
        depth_property.insert(
            "description".to_string(),
            Value::String(
                "Maximum depth to traverse for finding references (default: 1, maximum: 3)"
                    .to_string(),
            ),
        );
        depth_property.insert("default".to_string(), Value::Number(1.into()));
        depth_property.insert("minimum".to_string(), Value::Number(1.into()));
        depth_property.insert("maximum".to_string(), Value::Number(3.into()));

        // limit parameter
        let mut limit_property = JsonObject::new();
        limit_property.insert("type".to_string(), Value::String("number".to_string()));
        limit_property.insert(
            "description".to_string(),
            Value::String("The maximum number of results to return".to_string()),
        );
        limit_property.insert("default".to_string(), Value::Number(50.into()));
        limit_property.insert("minimum".to_string(), Value::Number(1.into()));
        limit_property.insert("maximum".to_string(), Value::Number(100.into()));
        properties.insert("limit".to_string(), Value::Object(limit_property));

        properties.insert("depth".to_string(), Value::Object(depth_property));

        let mut input_schema = JsonObject::new();
        input_schema.insert("type".to_string(), Value::String("object".to_string()));
        input_schema.insert("properties".to_string(), Value::Object(properties));
        input_schema.insert(
            "required".to_string(),
            Value::Array(vec![
                Value::String("absolute_file_path".to_string()),
                Value::String("symbol".to_string()),
            ]),
        );

        Tool {
            name: Cow::Borrowed(GET_SYMBOL_REFERENCES_TOOL_NAME),
            description: Some(Cow::Borrowed(GET_SYMBOL_REFERENCES_TOOL_DESCRIPTION)),
            input_schema: Arc::new(input_schema),
            output_schema: None,
            annotations: None,
        }
    }

    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let absolute_file_path = params
            .get("absolute_file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Missing absolute_file_path".to_string(),
                    None,
                )
            })?;

        let symbol = params
            .get("symbol_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Missing symbol".to_string(),
                    None,
                )
            })?;

        let depth = params
            .get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, 3) as u8;

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50)
            .clamp(1, 100) as u32;

        // Resolve workspace for the project
        let project_info = self
            .workspace_manager
            .get_project_for_file(absolute_file_path)
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "File not found in workspace manager".to_string(),
                    None,
                )
            })?;

        // Get database service
        let database = self
            .database
            .get_or_create_database(&project_info.database_path.to_string_lossy(), None)
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to get database for workspace".to_string(),
                    None,
                )
            })?;

        let service = NodeDatabaseService::new(&database);
        let relative_file_path = Path::new(absolute_file_path)
            .strip_prefix(&project_info.project_path)
            .unwrap();

        let mut references = Vec::new();
        for starting_symbol in self.search_initial_symbols(
            symbol,
            relative_file_path.to_str().unwrap(),
            project_info.database_path.clone(),
        ) {
            let mut visited = HashSet::new();

            let reference = self.find_references_recursive(
                &service,
                starting_symbol.fqn.as_str(),
                0,
                depth,
                limit, // Limit is the total number of references to return per symbol
                &mut visited,
            );

            if let Some(reference) = reference {
                references.push(reference);
            }
        }

        // Convert to JSON result
        let result = json!({
            "references": references.into_iter().map(|r| r.to_json()).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![
            Content::json(result).unwrap(),
        ]))
    }
}
